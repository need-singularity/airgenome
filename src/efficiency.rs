//! Efficiency — Banach contraction toward the 2/3 meta-fixed-point.
//!
//! Implements the contraction map `I_next = 0.7·I + 0.1` whose unique fixed
//! point is `1/3`. The efficiency score we track is the complement `2/3`
//! (the "work" fraction; `1/3` is system overhead). Discovered in the
//! `macbook-resource-gate` scan (score plateau at 0.64 ≈ 2/3).
//!
//! Also provides a lightweight mutual-information estimator over a short
//! history buffer, to drive the MiLens objective.

use serde::{Deserialize, Serialize};

/// Contraction constant from H-056 (`I_next = alpha·I + beta`).
pub const ALPHA: f64 = 0.7;
pub const BETA: f64 = 0.1;

/// Meta fixed point: 1/3. Derivation: `x = 0.7x + 0.1 ⇒ x = 1/3`.
pub const META_FP: f64 = 1.0 / 3.0;
/// Complement (work fraction): 2/3.
pub const WORK_FP: f64 = 2.0 / 3.0;

/// One iteration of the contraction map.
pub const fn contract_once(i: f64) -> f64 {
    ALPHA * i + BETA
}

/// Iterate the contraction map `n` times from `start`.
pub fn contract(start: f64, n: usize) -> f64 {
    let mut i = start;
    for _ in 0..n {
        i = contract_once(i);
    }
    i
}

/// Distance to the meta fixed point.
pub fn distance_to_fp(i: f64) -> f64 {
    (i - META_FP).abs()
}

/// True iff `|i - 1/3| < eps`.
pub fn converged(i: f64, eps: f64) -> bool {
    distance_to_fp(i) < eps
}

/// Efficiency tracker — keeps a short history and decides convergence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfficiencyTracker {
    /// Most recent samples, newest last.
    pub history: Vec<f64>,
    /// Maximum retained history length.
    pub capacity: usize,
    /// Convergence tolerance.
    pub eps: f64,
}

impl EfficiencyTracker {
    pub fn new(capacity: usize, eps: f64) -> Self {
        Self { history: Vec::with_capacity(capacity), capacity, eps }
    }

    pub fn push(&mut self, sample: f64) {
        if self.history.len() == self.capacity {
            self.history.remove(0);
        }
        self.history.push(sample);
    }

    /// Mean of the retained history, or `0.0` if empty.
    pub fn mean(&self) -> f64 {
        if self.history.is_empty() { return 0.0; }
        let s: f64 = self.history.iter().sum();
        s / self.history.len() as f64
    }

    /// True when the recent mean has settled at the 2/3 work fixed-point.
    pub fn converged_to_work_fp(&self) -> bool {
        (self.mean() - WORK_FP).abs() < self.eps
    }

    /// True when the recent mean has settled at the 1/3 meta fixed-point.
    pub fn converged_to_meta_fp(&self) -> bool {
        converged(self.mean(), self.eps)
    }
}

impl Default for EfficiencyTracker {
    fn default() -> Self { Self::new(16, 0.01) }
}

// ---- Mutual information (binning estimator) -------------------------------

/// Histogram-based mutual information `I(X;Y)` over paired samples.
///
/// Uses `bins × bins` discretization on `[min, max]` per variable. Returns
/// MI in nats (base e). `bins ≥ 2`, `xs.len() == ys.len() ≥ bins`.
pub fn mutual_info_hist(xs: &[f64], ys: &[f64], bins: usize) -> f64 {
    let n = xs.len().min(ys.len());
    if bins < 2 || n < bins {
        return 0.0;
    }

    let bin_of = |v: f64, lo: f64, hi: f64| -> usize {
        if hi <= lo { return 0; }
        let t = ((v - lo) / (hi - lo)).clamp(0.0, 0.999_999_999);
        (t * bins as f64) as usize
    };

    let (xlo, xhi) = range(xs);
    let (ylo, yhi) = range(ys);

    let mut joint = vec![0.0f64; bins * bins];
    let mut px = vec![0.0f64; bins];
    let mut py = vec![0.0f64; bins];

    for k in 0..n {
        let i = bin_of(xs[k], xlo, xhi);
        let j = bin_of(ys[k], ylo, yhi);
        joint[i * bins + j] += 1.0;
        px[i] += 1.0;
        py[j] += 1.0;
    }
    let n_f = n as f64;
    for v in joint.iter_mut() { *v /= n_f; }
    for v in px.iter_mut() { *v /= n_f; }
    for v in py.iter_mut() { *v /= n_f; }

    let mut mi = 0.0;
    for i in 0..bins {
        for j in 0..bins {
            let p = joint[i * bins + j];
            if p > 0.0 && px[i] > 0.0 && py[j] > 0.0 {
                mi += p * (p / (px[i] * py[j])).ln();
            }
        }
    }
    mi.max(0.0)
}

fn range(xs: &[f64]) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for &x in xs {
        if x < lo { lo = x; }
        if x > hi { hi = x; }
    }
    (lo, hi)
}

/// Per-pair MI gap: how much mutual information each gate fails to capture.
///
/// For each of the 15 axis pairs, computes MI between the two axes over
/// `history`, normalizes to `[0, 1]`, then subtracts the normalized fire
/// rate. Positive gap = "leaking gate" (correlated axes, rule not firing).
///
/// Returns `[f64; 15]` — one gap score per pair, clamped to `>= 0`.
pub fn mi_gap(
    history: &[crate::vitals::Vitals],
    fire_counts: &[usize; 15],
    bins: usize,
) -> [f64; 15] {
    use crate::gate::PAIRS;
    let n = history.len();
    if n < bins || bins < 2 {
        return [0.0; 15];
    }

    // Extract per-axis time series.
    let axis_series = |axis: crate::gate::Axis| -> Vec<f64> {
        history.iter().map(|v| v.get(axis)).collect()
    };

    // Compute MI for each pair.
    let mut mi_raw = [0.0f64; 15];
    for (k, &(a, b)) in PAIRS.iter().enumerate() {
        let xs = axis_series(a);
        let ys = axis_series(b);
        mi_raw[k] = mutual_info_hist(&xs, &ys, bins);
    }

    // Normalize MI to [0, 1].
    let mi_max = mi_raw.iter().cloned().fold(0.0f64, f64::max);
    let mi_norm: [f64; 15] = {
        let mut arr = [0.0; 15];
        for k in 0..15 {
            arr[k] = if mi_max > 0.0 { mi_raw[k] / mi_max } else { 0.0 };
        }
        arr
    };

    // Normalize fire rates to [0, 1].
    let fr_max = *fire_counts.iter().max().unwrap_or(&1) as f64;
    let fr_norm: [f64; 15] = {
        let mut arr = [0.0; 15];
        for k in 0..15 {
            arr[k] = if fr_max > 0.0 { fire_counts[k] as f64 / fr_max } else { 0.0 };
        }
        arr
    };

    // Gap = mi_norm - fr_norm, clamped to >= 0.
    let mut gaps = [0.0; 15];
    for k in 0..15 {
        gaps[k] = (mi_norm[k] - fr_norm[k]).max(0.0);
    }
    gaps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_point_values() {
        // x = 0.7x + 0.1 ⇒ x = 1/3
        assert!((META_FP - 1.0 / 3.0).abs() < f64::EPSILON);
        assert!((WORK_FP - 2.0 / 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn contraction_converges_to_one_third() {
        let out = contract(0.0, 100);
        assert!(distance_to_fp(out) < 1e-10);
        let out = contract(1.0, 100);
        assert!(distance_to_fp(out) < 1e-10);
    }

    #[test]
    fn contract_once_is_fixed_at_one_third() {
        let x = contract_once(META_FP);
        assert!((x - META_FP).abs() < 1e-15);
    }

    #[test]
    fn converged_tolerates_eps() {
        assert!(converged(0.333, 0.01));
        assert!(!converged(0.5, 0.01));
    }

    #[test]
    fn tracker_respects_capacity() {
        let mut t = EfficiencyTracker::new(3, 0.01);
        t.push(1.0); t.push(2.0); t.push(3.0); t.push(4.0);
        assert_eq!(t.history, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn tracker_detects_work_fp() {
        let mut t = EfficiencyTracker::new(4, 0.01);
        for _ in 0..4 { t.push(WORK_FP); }
        assert!(t.converged_to_work_fp());
        assert!(!t.converged_to_meta_fp());
    }

    #[test]
    fn mi_zero_for_independent() {
        // Constant ys → zero MI.
        let xs: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let ys: Vec<f64> = vec![1.0; 100];
        assert!(mutual_info_hist(&xs, &ys, 5) < 1e-9);
    }

    #[test]
    fn mi_positive_for_dependent() {
        let xs: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let ys: Vec<f64> = xs.iter().map(|x| 2.0 * x + 3.0).collect();
        let mi = mutual_info_hist(&xs, &ys, 5);
        assert!(mi > 0.5, "expected strong MI, got {mi}");
    }

    #[test]
    fn mi_gap_zero_when_fire_matches_mi() {
        use crate::gate::Axis;
        // Construct vitals where cpu and ram are perfectly correlated
        // AND fire_counts reflect that — gap should be ~0 for pair 0.
        let mut history = Vec::new();
        for i in 0..100 {
            let t = i as f64 / 100.0;
            let mut axes = [0.0; 6];
            axes[Axis::Cpu.index()] = 5.0 * t;
            axes[Axis::Ram.index()] = t;
            axes[Axis::Gpu.index()] = 8.0;
            axes[Axis::Npu.index()] = 8.0;
            axes[Axis::Power.index()] = 1.0;
            axes[Axis::Io.index()] = 0.0;
            history.push(crate::vitals::Vitals { ts: i, axes });
        }
        // fire_counts[0] = 100 (always fires) → fire_rate = 1.0
        let mut fire_counts = [0usize; 15];
        fire_counts[0] = 100;
        let gaps = mi_gap(&history, &fire_counts, 10);
        // gap[0] should be 0 or negative (clamped to 0)
        assert!(gaps[0] < 0.1, "expected near-zero gap, got {}", gaps[0]);
    }

    #[test]
    fn mi_gap_positive_when_correlated_but_never_fires() {
        use crate::gate::Axis;
        let mut history = Vec::new();
        for i in 0..100 {
            let t = i as f64 / 100.0;
            let mut axes = [0.0; 6];
            axes[Axis::Cpu.index()] = 5.0 * t;
            axes[Axis::Ram.index()] = t;  // correlated with cpu
            axes[Axis::Gpu.index()] = 8.0;
            axes[Axis::Npu.index()] = 8.0;
            axes[Axis::Power.index()] = 1.0;
            axes[Axis::Io.index()] = 0.0;
            history.push(crate::vitals::Vitals { ts: i, axes });
        }
        // fire_counts[0] = 0 — never fires despite high MI
        let fire_counts = [0usize; 15];
        let gaps = mi_gap(&history, &fire_counts, 10);
        // MI(cpu, ram) is high, fire_rate = 0 → gap > 0
        assert!(gaps[0] > 0.3, "expected positive gap for correlated pair, got {}", gaps[0]);
    }

    #[test]
    fn mi_gap_has_fifteen_entries() {
        let history = vec![crate::vitals::Vitals::zeroed(); 20];
        let fire_counts = [0usize; 15];
        let gaps = mi_gap(&history, &fire_counts, 5);
        assert_eq!(gaps.len(), 15);
    }
}
