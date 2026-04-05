//! VitalsBuffer — ring buffer of recent vitals with preemptive triggers.
//!
//! Implements three of the four v0.5.0 breakthrough principles:
//!
//! 1. **Preemptive** — derivative-based triggers (`d(axis)/dt`)
//! 2. **Ratio** — axis-pair ratios with thresholds
//! 3. **Oscillation filter** — detect on/off toggling over recent window
//!
//! (The fourth principle, *inverse pairs*, already lives in [`crate::actuator`].)

use crate::gate::{Axis, PAIR_COUNT};
use crate::vitals::Vitals;

/// Fixed-capacity ring buffer of recent vitals samples.
#[derive(Debug, Clone)]
pub struct VitalsBuffer {
    samples: Vec<Vitals>,
    capacity: usize,
}

impl VitalsBuffer {
    /// Create a buffer retaining the last `capacity` samples.
    ///
    /// Panics if `capacity < 2`.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity >= 2, "buffer capacity must be >= 2");
        Self { samples: Vec::with_capacity(capacity), capacity }
    }

    pub fn push(&mut self, v: Vitals) {
        if self.samples.len() == self.capacity {
            self.samples.remove(0);
        }
        self.samples.push(v);
    }

    pub fn len(&self) -> usize { self.samples.len() }
    pub fn is_empty(&self) -> bool { self.samples.is_empty() }
    pub fn capacity(&self) -> usize { self.capacity }
    pub fn latest(&self) -> Option<&Vitals> { self.samples.last() }
    pub fn oldest(&self) -> Option<&Vitals> { self.samples.first() }

    /// Per-axis rate of change between the oldest and newest retained
    /// sample, in units-per-second.
    ///
    /// Returns `None` if fewer than two samples are present OR if the
    /// two span zero seconds.
    pub fn derivative(&self, axis: Axis) -> Option<f64> {
        if self.samples.len() < 2 { return None; }
        let first = self.samples.first()?;
        let last = self.samples.last()?;
        let dt = last.ts.saturating_sub(first.ts) as f64;
        if dt <= 0.0 { return None; }
        Some((last.get(axis) - first.get(axis)) / dt)
    }

    /// True when `|d(axis)/dt| >= threshold` AND the derivative points in
    /// `sign` direction (positive → rising, negative → falling, 0 → any).
    pub fn preemptive(&self, axis: Axis, threshold: f64, sign: i8) -> bool {
        let Some(d) = self.derivative(axis) else { return false; };
        if d.abs() < threshold { return false; }
        match sign {
            s if s > 0 => d > 0.0,
            s if s < 0 => d < 0.0,
            _ => true,
        }
    }

    /// Ratio between two axes on the latest sample.
    /// Returns `None` if the denominator is zero or no samples retained.
    pub fn ratio(&self, num: Axis, denom: Axis) -> Option<f64> {
        let v = self.samples.last()?;
        let d = v.get(denom);
        if d.abs() < f64::EPSILON { return None; }
        Some(v.get(num) / d)
    }

    /// Count sign-changes of `d(axis)/dt` across consecutive sample pairs.
    /// A high count over a short window indicates oscillation.
    pub fn oscillation_count(&self, axis: Axis) -> usize {
        if self.samples.len() < 3 { return 0; }
        let mut count = 0usize;
        let mut prev_sign: i8 = 0;
        for w in self.samples.windows(2) {
            let dt = w[1].ts.saturating_sub(w[0].ts) as f64;
            if dt <= 0.0 { continue; }
            let slope = (w[1].get(axis) - w[0].get(axis)) / dt;
            let sign: i8 = if slope > 0.0 { 1 } else if slope < 0.0 { -1 } else { 0 };
            if sign != 0 && prev_sign != 0 && sign != prev_sign {
                count += 1;
            }
            if sign != 0 { prev_sign = sign; }
        }
        count
    }

    /// True when `axis` oscillates more than `max_flips` times in the
    /// current window. Use this to lock out a pair gate that would
    /// toggle ON/OFF repeatedly.
    pub fn oscillating(&self, axis: Axis, max_flips: usize) -> bool {
        self.oscillation_count(axis) > max_flips
    }
}

/// Bitmask of the fifteen pair gates (low 15 bits).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GateMask(pub u16);

impl GateMask {
    pub const fn empty() -> Self { GateMask(0) }
    pub const fn full() -> Self { GateMask(0x7FFF) } // bits 0..14 set

    pub fn engage(mut self, k: usize) -> Self {
        if k < PAIR_COUNT { self.0 |= 1u16 << k; }
        self
    }
    pub fn disengage(mut self, k: usize) -> Self {
        if k < PAIR_COUNT { self.0 &= !(1u16 << k); }
        self
    }
    pub const fn is_engaged(self, k: usize) -> bool {
        if k >= PAIR_COUNT { return false; }
        (self.0 >> k) & 1 == 1
    }
    pub fn count(self) -> usize { self.0.count_ones() as usize }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vitals(ts: u64, cpu: f64, ram: f64) -> Vitals {
        let mut axes = [0.0; 6];
        axes[Axis::Cpu.index()] = cpu;
        axes[Axis::Ram.index()] = ram;
        Vitals { ts, axes }
    }

    #[test]
    #[should_panic]
    fn capacity_below_two_panics() { let _ = VitalsBuffer::new(1); }

    #[test]
    fn push_respects_capacity() {
        let mut b = VitalsBuffer::new(3);
        for t in 0..5 { b.push(vitals(t, 0.0, 0.0)); }
        assert_eq!(b.len(), 3);
        assert_eq!(b.oldest().unwrap().ts, 2);
        assert_eq!(b.latest().unwrap().ts, 4);
    }

    #[test]
    fn derivative_is_zero_on_flat_signal() {
        let mut b = VitalsBuffer::new(5);
        for t in 0..5 { b.push(vitals(t * 10, 1.0, 0.5)); }
        assert_eq!(b.derivative(Axis::Cpu), Some(0.0));
        assert_eq!(b.derivative(Axis::Ram), Some(0.0));
    }

    #[test]
    fn derivative_is_units_per_second() {
        let mut b = VitalsBuffer::new(3);
        b.push(vitals(0, 1.0, 0.5));
        b.push(vitals(10, 1.5, 0.8));   // ram +0.3 over 10 s
        b.push(vitals(20, 2.0, 1.0));   // overall: 0.5 over 20s
        let d = b.derivative(Axis::Ram).unwrap();
        assert!((d - 0.025).abs() < 1e-9, "got {d}");
    }

    #[test]
    fn derivative_needs_two_samples_and_nonzero_span() {
        let mut b = VitalsBuffer::new(5);
        assert!(b.derivative(Axis::Cpu).is_none());
        b.push(vitals(10, 1.0, 0.5));
        assert!(b.derivative(Axis::Cpu).is_none());
        b.push(vitals(10, 1.0, 0.5));  // same ts → dt=0
        assert!(b.derivative(Axis::Cpu).is_none());
    }

    #[test]
    fn preemptive_fires_on_rising_fast_enough() {
        let mut b = VitalsBuffer::new(3);
        b.push(vitals(0,  0.0, 0.0));
        b.push(vitals(10, 0.0, 1.0));   // d(ram)/dt = 0.1
        assert!(b.preemptive(Axis::Ram, 0.05, 1));
        assert!(!b.preemptive(Axis::Ram, 0.2, 1));
        assert!(!b.preemptive(Axis::Ram, 0.05, -1)); // wrong sign
    }

    #[test]
    fn ratio_computes_on_latest_sample() {
        let mut b = VitalsBuffer::new(3);
        b.push(vitals(0, 4.0, 0.5));
        b.push(vitals(10, 2.0, 0.1));
        assert_eq!(b.ratio(Axis::Cpu, Axis::Ram), Some(20.0));
    }

    #[test]
    fn ratio_returns_none_on_zero_denom() {
        let mut b = VitalsBuffer::new(3);
        b.push(vitals(0, 1.0, 0.0));
        assert_eq!(b.ratio(Axis::Cpu, Axis::Ram), None);
    }

    #[test]
    fn oscillation_count_on_sawtooth() {
        let mut b = VitalsBuffer::new(6);
        for (i, ts) in (0..6).enumerate() {
            let v = if i % 2 == 0 { 1.0 } else { 0.0 };
            b.push(vitals(ts, v, 0.0));
        }
        // cpu alternates → 4 consecutive sign changes (between 5 slopes)
        assert!(b.oscillation_count(Axis::Cpu) >= 3);
        assert!(b.oscillating(Axis::Cpu, 2));
        assert!(!b.oscillating(Axis::Cpu, 10));
    }

    #[test]
    fn oscillation_zero_on_monotone() {
        let mut b = VitalsBuffer::new(5);
        for t in 0..5 { b.push(vitals(t, t as f64, 0.0)); }
        assert_eq!(b.oscillation_count(Axis::Cpu), 0);
    }

    #[test]
    fn gate_mask_ops() {
        let m = GateMask::empty().engage(0).engage(7).engage(14);
        assert_eq!(m.count(), 3);
        assert!(m.is_engaged(0));
        assert!(m.is_engaged(7));
        assert!(m.is_engaged(14));
        assert!(!m.is_engaged(1));
        let m = m.disengage(7);
        assert_eq!(m.count(), 2);
        assert!(!m.is_engaged(7));
    }

    #[test]
    fn gate_mask_full_has_fifteen_bits() {
        assert_eq!(GateMask::full().count(), PAIR_COUNT);
        for k in 0..PAIR_COUNT { assert!(GateMask::full().is_engaged(k)); }
    }

    #[test]
    fn gate_mask_ignores_out_of_range() {
        let m = GateMask::empty().engage(99);
        assert_eq!(m.count(), 0);
        assert!(!GateMask::full().is_engaged(99));
    }
}
