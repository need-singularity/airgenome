//! PolicyEngine — wires [`VitalsBuffer`] to the 15 rules with the three
//! filters from v0.5.0 (preemptive, ratio, oscillation).
//!
//! The engine is still **dry-run**: every `.tick()` returns a list of
//! proposed action strings, never touches the system. The caller decides
//! whether to print them, log them, or (in a future `apply_live` path)
//! execute them.

use crate::buffer::VitalsBuffer;
use crate::gate::{Axis, PAIR_COUNT};
use crate::rules::{fires, RULES};
use crate::vitals::Vitals;

/// One policy decision for a single pair gate.
#[derive(Debug, Clone, PartialEq)]
pub struct Proposal {
    pub pair: usize,
    pub reason: Reason,
    pub action: &'static str,
}

/// Why the proposal was generated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Static threshold rule fired on current vitals.
    Reactive,
    /// A derivative crossed its rate threshold.
    Preemptive,
}

/// Configuration knobs for the engine.
#[derive(Debug, Clone, Copy)]
pub struct PolicyConfig {
    /// Minimum samples retained in the buffer before derivatives are trusted.
    pub min_samples: usize,
    /// `|d(ram)/dt|` above this (per second) triggers a preemptive RAM event.
    pub ram_rise_threshold: f64,
    /// Same for CPU.
    pub cpu_rise_threshold: f64,
    /// Sawtooth flips allowed in the window before a pair is locked out.
    pub max_oscillations: usize,
    /// Cooldown after a pair has fired — that pair is suppressed for this
    /// many ticks.
    pub cooldown_ticks: usize,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            min_samples: 3,
            ram_rise_threshold: 0.02,   // 0.02 / sec pressure climb
            cpu_rise_threshold: 0.5,    // 0.5 / sec load climb
            max_oscillations: 2,
            cooldown_ticks: 3,
        }
    }
}

/// The engine.
#[derive(Debug, Clone)]
pub struct PolicyEngine {
    buffer: VitalsBuffer,
    cfg: PolicyConfig,
    /// Remaining cooldown ticks per pair (0 = ready).
    cooldown: [usize; PAIR_COUNT],
}

impl PolicyEngine {
    pub fn new(buffer_capacity: usize, cfg: PolicyConfig) -> Self {
        Self {
            buffer: VitalsBuffer::new(buffer_capacity),
            cfg,
            cooldown: [0; PAIR_COUNT],
        }
    }

    pub fn with_defaults(buffer_capacity: usize) -> Self {
        Self::new(buffer_capacity, PolicyConfig::default())
    }

    pub fn buffer(&self) -> &VitalsBuffer { &self.buffer }
    pub fn cfg(&self) -> &PolicyConfig { &self.cfg }

    /// Push one vitals sample and return the proposals emitted this tick.
    ///
    /// A pair may emit at most one proposal per tick, and none while in
    /// cooldown. Oscillating pairs are fully suppressed.
    pub fn tick(&mut self, v: Vitals) -> Vec<Proposal> {
        self.buffer.push(v);
        // decrement cooldowns
        for c in self.cooldown.iter_mut() { if *c > 0 { *c -= 1; } }

        if self.buffer.len() < self.cfg.min_samples {
            return vec![];
        }

        let mut out = Vec::new();
        let latest = match self.buffer.latest() { Some(v) => *v, None => return out };

        // oscillation lockout: suppress pairs whose dominant axis is flapping
        let cpu_osc = self.buffer.oscillating(Axis::Cpu, self.cfg.max_oscillations);
        let ram_osc = self.buffer.oscillating(Axis::Ram, self.cfg.max_oscillations);

        // preemptive flags
        let ram_rising = self.buffer.preemptive(
            Axis::Ram, self.cfg.ram_rise_threshold, 1);
        let cpu_rising = self.buffer.preemptive(
            Axis::Cpu, self.cfg.cpu_rise_threshold, 1);

        for k in 0..PAIR_COUNT {
            if self.cooldown[k] > 0 { continue; }

            // skip CPU/RAM-centered pairs if their dominant axis oscillates
            let suppress = (pair_uses(k, Axis::Cpu) && cpu_osc)
                        || (pair_uses(k, Axis::Ram) && ram_osc);
            if suppress { continue; }

            // reactive check against static thresholds
            let react = fires(k, &latest);

            // preemptive: RAM-centered pairs fire earlier when ram_rising
            let preempt = (pair_uses(k, Axis::Ram) && ram_rising)
                       || (pair_uses(k, Axis::Cpu) && cpu_rising);

            if react {
                out.push(Proposal {
                    pair: k, reason: Reason::Reactive, action: RULES[k].action,
                });
                self.cooldown[k] = self.cfg.cooldown_ticks;
            } else if preempt {
                out.push(Proposal {
                    pair: k, reason: Reason::Preemptive, action: RULES[k].action,
                });
                self.cooldown[k] = self.cfg.cooldown_ticks;
            }
        }
        out
    }
}

fn pair_uses(k: usize, axis: Axis) -> bool {
    use crate::gate::PAIRS;
    if k >= PAIR_COUNT { return false; }
    let (a, b) = PAIRS[k];
    a == axis || b == axis
}

/// Cascade result from mesh neighbor analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CascadeInfo {
    pub pair: usize,
    pub neighbor_fires: u8,
    pub boost: u8,
}

/// Compute mesh cascade for a set of fired pairs.
///
/// For each pair in `fired`, counts how many of its 3 mesh neighbors
/// also fired. Returns a `CascadeInfo` per fired pair.
///   - 0-1 neighbors: boost = 0 (no cascade)
///   - 2 neighbors:   boost = 0x10 (stability increment)
///   - 3 neighbors:   boost = 0x20 (full cascade + surprise)
pub fn mesh_cascade_for(fired: &[usize], _v: &Vitals) -> Vec<CascadeInfo> {
    use crate::rules::neighbors;
    fired.iter().map(|&k| {
        let ns = neighbors(k);
        let count = ns.iter().filter(|n| fired.contains(n)).count() as u8;
        let boost = match count {
            0 | 1 => 0,
            2 => 0x10,
            _ => 0x20,
        };
        CascadeInfo { pair: k, neighbor_fires: count, boost }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(ts: u64, cpu: f64, ram: f64, io: f64) -> Vitals {
        let mut axes = [0.0; 6];
        axes[Axis::Cpu.index()] = cpu;
        axes[Axis::Ram.index()] = ram;
        axes[Axis::Gpu.index()] = 8.0;
        axes[Axis::Npu.index()] = 8.0;
        axes[Axis::Power.index()] = 1.0;
        axes[Axis::Io.index()] = io;
        Vitals { ts, axes }
    }

    #[test]
    fn default_config_is_sane() {
        let c = PolicyConfig::default();
        assert!(c.min_samples >= 2);
        assert!(c.cooldown_ticks >= 1);
        assert!(c.ram_rise_threshold > 0.0);
        assert!(c.max_oscillations >= 1);
    }

    #[test]
    fn tick_returns_empty_until_min_samples() {
        let mut p = PolicyEngine::with_defaults(10);
        assert!(p.tick(v(0, 0.0, 0.0, 0.0)).is_empty());
        assert!(p.tick(v(1, 0.0, 0.0, 0.0)).is_empty());
        // 3rd sample: buffer hits min_samples, but vitals idle → no reactive fires
        let out = p.tick(v(2, 0.0, 0.0, 0.0));
        // gpu×npu rule 9 always fires (structural)
        assert!(out.iter().any(|p| p.pair == 9));
    }

    #[test]
    fn reactive_fires_on_high_load() {
        let mut p = PolicyEngine::with_defaults(10);
        // 2 warm-up pushes, then 3rd tick is the first with enough samples.
        p.tick(v(0, 5.0, 0.95, 2.0));
        p.tick(v(1, 5.0, 0.95, 2.0));
        let out = p.tick(v(2, 5.0, 0.95, 2.0));
        assert!(out.iter().any(|p| p.pair == 0 && p.reason == Reason::Reactive));
    }

    #[test]
    fn cooldown_suppresses_repeat_fires() {
        let mut cfg = PolicyConfig::default();
        cfg.cooldown_ticks = 3;
        let mut p = PolicyEngine::new(10, cfg);
        p.tick(v(0, 5.0, 0.95, 2.0));
        p.tick(v(1, 5.0, 0.95, 2.0));
        let first = p.tick(v(2, 5.0, 0.95, 2.0));  // first tick with proposals
        assert!(first.iter().any(|p| p.pair == 0));
        // within cooldown window → pair 0 should not fire again
        let second = p.tick(v(3, 5.0, 0.95, 2.0));
        assert!(!second.iter().any(|p| p.pair == 0));
    }

    #[test]
    fn preemptive_fires_on_rising_ram() {
        let mut p = PolicyEngine::with_defaults(10);
        // ram climbs from 0.1 → 0.3 over 5s → d/dt = 0.04/s > 0.02 threshold
        p.tick(v(0, 1.0, 0.10, 0.0));
        p.tick(v(2, 1.0, 0.18, 0.0));
        let out = p.tick(v(5, 1.0, 0.30, 0.0));
        // ram-centered pairs should have at least one preemptive proposal
        // (ram values < 0.80 → reactive rules don't fire)
        assert!(out.iter().any(|p| p.reason == Reason::Preemptive));
    }

    #[test]
    fn oscillation_suppresses_ram_pairs() {
        let mut p = PolicyEngine::with_defaults(10);
        // ram oscillates high/low high/low...
        for t in 0..6 {
            let ram = if t % 2 == 0 { 0.95 } else { 0.10 };
            p.tick(v(t, 5.0, ram, 0.0));
        }
        // final tick: ram high, would normally fire pair 0, but oscillating
        let out = p.tick(v(10, 5.0, 0.95, 0.0));
        // With ram oscillating, pair 0 (cpu×ram) is suppressed.
        // (cpu also oscillates? no — cpu is flat at 5.0 → not oscillating)
        // So cpu×io etc may still fire. But ram-centered pairs should not.
        assert!(!out.iter().any(|p| p.pair == 0));
        assert!(!out.iter().any(|p| p.pair == 7)); // ram×power
    }

    #[test]
    fn pair_uses_matches_canonical_pairs() {
        // pair 0 = cpu×ram
        assert!(pair_uses(0, Axis::Cpu));
        assert!(pair_uses(0, Axis::Ram));
        assert!(!pair_uses(0, Axis::Gpu));
        // pair 14 = power×io
        assert!(pair_uses(14, Axis::Power));
        assert!(pair_uses(14, Axis::Io));
        assert!(!pair_uses(14, Axis::Cpu));
    }

    #[test]
    fn gpu_npu_structural_rule_is_always_reactive() {
        let mut p = PolicyEngine::with_defaults(5);
        p.tick(v(0, 0.1, 0.1, 0.0));
        p.tick(v(1, 0.1, 0.1, 0.0));
        // third tick: first with enough samples → rule 9 fires immediately
        let out = p.tick(v(2, 0.1, 0.1, 0.0));
        assert!(out.iter().any(|p| p.pair == 9 && p.reason == Reason::Reactive));
    }

    #[test]
    fn cascade_no_neighbors_firing() {
        let fired = vec![0]; // only pair 0 fires
        // neighbors(0) = [1, 5, 11] — none of them in `fired`
        let v_idle = v(0, 0.5, 0.1, 0.0);
        let cascades = mesh_cascade_for(&fired, &v_idle);
        assert_eq!(cascades.len(), 1);
        assert_eq!(cascades[0].pair, 0);
        assert_eq!(cascades[0].neighbor_fires, 0);
        assert_eq!(cascades[0].boost, 0);
    }

    #[test]
    fn cascade_two_neighbors_firing() {
        // pair 0 fires, neighbors(0) = [1, 5, 11]
        // if pairs 1 and 5 also fire → 2 neighbors → boost 0x10
        let fired = vec![0, 1, 5];
        let v_idle = v(0, 0.5, 0.1, 0.0);
        let cascades = mesh_cascade_for(&fired, &v_idle);
        let c0 = cascades.iter().find(|c| c.pair == 0).unwrap();
        assert_eq!(c0.neighbor_fires, 2);
        assert_eq!(c0.boost, 0x10);
    }

    #[test]
    fn cascade_full_three_neighbors() {
        // pair 0 fires, all neighbors [1, 5, 11] also fire → boost 0x20
        let fired = vec![0, 1, 5, 11];
        let v_idle = v(0, 0.5, 0.1, 0.0);
        let cascades = mesh_cascade_for(&fired, &v_idle);
        let c0 = cascades.iter().find(|c| c.pair == 0).unwrap();
        assert_eq!(c0.neighbor_fires, 3);
        assert_eq!(c0.boost, 0x20);
    }
}
