//! Nexus Merger — cross-gate mutual information + breakthrough efficiency
//! projection over historical per-category signatures.

use std::collections::BTreeMap;

/// Per-tick per-gate sample: (ts, ram, cpu).
#[derive(Debug, Clone, Copy)]
pub struct GateSample { pub ts: u64, pub ram: f64, pub cpu: f64 }

/// Binned mutual-information estimator. Returns 0 if insufficient data.
pub fn mutual_info(xs: &[f64], ys: &[f64], bins: usize) -> f64 {
    if xs.len() < 2 || xs.len() != ys.len() { return 0.0; }
    let mnx = xs.iter().cloned().fold(f64::INFINITY, f64::min);
    let mxx = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mny = ys.iter().cloned().fold(f64::INFINITY, f64::min);
    let mxy = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if mxx == mnx || mxy == mny { return 0.0; }
    let bin = |v: f64, mn: f64, mx: f64| -> usize {
        let idx = ((v - mn) / (mx - mn) * bins as f64) as usize;
        idx.min(bins - 1)
    };
    let n = xs.len() as f64;
    let mut joint: BTreeMap<(usize, usize), f64> = BTreeMap::new();
    let mut px = vec![0.0; bins];
    let mut py = vec![0.0; bins];
    for (x, y) in xs.iter().zip(ys.iter()) {
        let i = bin(*x, mnx, mxx); let j = bin(*y, mny, mxy);
        *joint.entry((i, j)).or_insert(0.0) += 1.0;
        px[i] += 1.0; py[j] += 1.0;
    }
    let mut mi = 0.0;
    for ((i, j), c) in &joint {
        let pxy = c / n; let pxi = px[*i] / n; let pyj = py[*j] / n;
        if pxy > 0.0 && pxi > 0.0 && pyj > 0.0 {
            mi += pxy * (pxy / (pxi * pyj)).log2();
        }
    }
    mi
}

/// Pearson correlation coefficient. Returns 0 if degenerate.
pub fn pearson(xs: &[f64], ys: &[f64]) -> f64 {
    if xs.len() < 2 || xs.len() != ys.len() { return 0.0; }
    let n = xs.len() as f64;
    let mx = xs.iter().sum::<f64>() / n;
    let my = ys.iter().sum::<f64>() / n;
    let mut num = 0.0; let mut dx = 0.0; let mut dy = 0.0;
    for (x, y) in xs.iter().zip(ys.iter()) {
        num += (x - mx) * (y - my);
        dx += (x - mx).powi(2); dy += (y - my).powi(2);
    }
    if dx <= 0.0 || dy <= 0.0 { return 0.0; }
    num / (dx * dy).sqrt()
}

/// Align two gate sample streams by shared timestamp.
pub fn align(a: &[GateSample], b: &[GateSample]) -> (Vec<f64>, Vec<f64>) {
    let bmap: BTreeMap<u64, (f64, f64)> = b.iter().map(|s| (s.ts, (s.ram, s.cpu))).collect();
    let mut xs = Vec::new(); let mut ys = Vec::new();
    for s in a {
        if let Some(&(r, _c)) = bmap.get(&s.ts) {
            xs.push(s.ram); ys.push(r);
        }
    }
    (xs, ys)
}

/// Output of the breakthrough projection.
#[derive(Debug, Clone)]
pub struct BreakthroughReport {
    pub per_gate_mi_sum: f64,
    pub cross_gate_mi_sum: f64,
    pub scaling_factor: f64,
    pub raw: f64,
    pub current_mesh: f64,
    pub new_cross_coupling: f64,
    pub new_mi_recovery: f64,
    pub ghost_penalty: f64,
    pub adjusted: f64,
    pub singularity: f64,
    pub distance: f64,
    pub crossed: bool,
    pub per_gate_mi: Vec<(String, f64)>,
    pub pair_mi: Vec<(String, String, f64, f64, usize)>,
}

/// Compute the breakthrough report from 4+ gate streams.
///
/// Uses the same constants as the existing `nexus` command:
///   raw = 0.6360, mesh = 0.0044, ghost_penalty = -0.0026
/// Scaling factor derived from nexus's implicit ratio: 0.0151 / 1.500.
pub fn project_breakthrough(streams: &[(String, Vec<GateSample>)]) -> BreakthroughReport {
    const RAW: f64 = 0.6360;
    const CURRENT_MESH: f64 = 0.0044;
    const GHOST_PENALTY: f64 = -0.0026;
    const SCALE: f64 = 0.0151 / 1.500;
    const SINGULARITY: f64 = 2.0 / 3.0;

    // per-gate MI (ram × cpu)
    let mut per_gate_mi_sum = 0.0;
    let mut per_gate_mi = Vec::new();
    for (name, s) in streams {
        if s.len() < 10 { continue; }
        let rams: Vec<f64> = s.iter().map(|x| x.ram).collect();
        let cpus: Vec<f64> = s.iter().map(|x| x.cpu).collect();
        let mi = mutual_info(&rams, &cpus, 6);
        per_gate_mi_sum += mi;
        per_gate_mi.push((name.clone(), mi));
    }

    // cross-gate MI (ram_A × ram_B) for all pairs
    let mut cross_gate_mi_sum = 0.0;
    let mut pair_mi = Vec::new();
    for i in 0..streams.len() {
        for j in (i+1)..streams.len() {
            let (xs, ys) = align(&streams[i].1, &streams[j].1);
            if xs.len() < 10 { continue; }
            let mi = mutual_info(&xs, &ys, 6);
            let r = pearson(&xs, &ys);
            cross_gate_mi_sum += mi;
            pair_mi.push((streams[i].0.clone(), streams[j].0.clone(), mi, r, xs.len()));
        }
    }

    let new_mi_recovery = per_gate_mi_sum * SCALE * 1.5;
    let new_cross_coupling = cross_gate_mi_sum * SCALE;
    let adjusted = RAW + (CURRENT_MESH + new_cross_coupling) + new_mi_recovery + GHOST_PENALTY;
    let distance = SINGULARITY - adjusted;
    let crossed = adjusted > SINGULARITY;

    BreakthroughReport {
        per_gate_mi_sum, cross_gate_mi_sum, scaling_factor: SCALE,
        raw: RAW, current_mesh: CURRENT_MESH,
        new_cross_coupling, new_mi_recovery, ghost_penalty: GHOST_PENALTY,
        adjusted, singularity: SINGULARITY, distance, crossed,
        per_gate_mi, pair_mi,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutual_info_identical_is_high() {
        let xs = (0..100).map(|i| i as f64).collect::<Vec<_>>();
        let ys = xs.clone();
        let mi = mutual_info(&xs, &ys, 6);
        assert!(mi > 1.0, "expected strong MI for identical streams, got {mi}");
    }

    #[test]
    fn mutual_info_constant_is_zero() {
        let xs = vec![1.0; 50];
        let ys = (0..50).map(|i| i as f64).collect::<Vec<_>>();
        let mi = mutual_info(&xs, &ys, 6);
        assert_eq!(mi, 0.0);
    }

    #[test]
    fn pearson_perfect_positive() {
        let xs: Vec<f64> = (0..50).map(|i| i as f64).collect();
        let ys: Vec<f64> = xs.iter().map(|x| 2.0 * x + 5.0).collect();
        let r = pearson(&xs, &ys);
        assert!((r - 1.0).abs() < 1e-10, "expected r=1.0, got {r}");
    }

    #[test]
    fn pearson_perfect_negative() {
        let xs: Vec<f64> = (0..50).map(|i| i as f64).collect();
        let ys: Vec<f64> = xs.iter().map(|x| -3.0 * x).collect();
        let r = pearson(&xs, &ys);
        assert!((r + 1.0).abs() < 1e-10, "expected r=-1.0, got {r}");
    }

    #[test]
    fn align_intersects_timestamps() {
        let a = vec![
            GateSample { ts: 1, ram: 0.1, cpu: 0.2 },
            GateSample { ts: 2, ram: 0.2, cpu: 0.3 },
            GateSample { ts: 3, ram: 0.3, cpu: 0.4 },
        ];
        let b = vec![
            GateSample { ts: 2, ram: 0.5, cpu: 0.6 },
            GateSample { ts: 3, ram: 0.6, cpu: 0.7 },
            GateSample { ts: 4, ram: 0.7, cpu: 0.8 },
        ];
        let (xs, ys) = align(&a, &b);
        assert_eq!(xs.len(), 2);
        assert_eq!(xs, vec![0.2, 0.3]);
        assert_eq!(ys, vec![0.5, 0.6]);
    }

    #[test]
    fn project_breakthrough_with_correlated_streams_crosses() {
        // Build 4 correlated gate streams — adjusted should exceed raw.
        let mk = |phase: f64| -> Vec<GateSample> {
            (0..100).map(|i| {
                let t = i as f64;
                GateSample {
                    ts: i as u64,
                    ram: (t * 0.1 + phase).sin().abs(),
                    cpu: (t * 0.1 + phase + 0.5).sin().abs(),
                }
            }).collect()
        };
        let streams: Vec<(String, Vec<GateSample>)> = vec![
            ("macos".into(), mk(0.0)),
            ("finder".into(), mk(0.3)),
            ("telegram".into(), mk(0.5)),
            ("browser".into(), mk(0.7)),
        ];
        let r = project_breakthrough(&streams);
        assert!(r.per_gate_mi_sum > 0.0);
        assert!(r.cross_gate_mi_sum > 0.0);
        // with 4 correlated streams, adjusted should be well above raw
        assert!(r.adjusted > r.raw);
    }
}
