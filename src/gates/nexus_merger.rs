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

/// Lagged mutual information: MI(xs[:-lag], ys[lag:]).
/// Positive lag means "does past A predict future B" (A → B causality hint).
pub fn lagged_mutual_info(xs: &[f64], ys: &[f64], lag: usize, bins: usize) -> f64 {
    if lag == 0 { return mutual_info(xs, ys, bins); }
    if xs.len() <= lag || ys.len() <= lag { return 0.0; }
    let a = &xs[..xs.len() - lag];
    let b = &ys[lag..];
    mutual_info(a, b, bins)
}

/// Default lag steps for temporal breakthrough analysis.
pub const LAG_STEPS: &[usize] = &[1, 2, 5, 10];

/// Triadic interaction information I(A;B;C).
///
/// Positive = redundancy (B, C overlap in predicting A).
/// Negative = synergy (A, B, C together reveal more than pairs).
/// Zero = full independence or fully captured by pairwise.
///
/// Formula:
///   I(X;Y;Z) = Σ p(x,y,z) · log2[ p(x,y,z)·p(x)·p(y)·p(z)
///                                  / (p(x,y)·p(x,z)·p(y,z)) ]
///
/// Uses bins=4 (triadic histograms are sparse; 4^3 = 64 cells).
pub fn triadic_interaction(xs: &[f64], ys: &[f64], zs: &[f64], bins: usize) -> f64 {
    if xs.len() < 15 || xs.len() != ys.len() || xs.len() != zs.len() { return 0.0; }
    let mnx = xs.iter().cloned().fold(f64::INFINITY, f64::min);
    let mxx = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mny = ys.iter().cloned().fold(f64::INFINITY, f64::min);
    let mxy = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mnz = zs.iter().cloned().fold(f64::INFINITY, f64::min);
    let mxz = zs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if mxx == mnx || mxy == mny || mxz == mnz { return 0.0; }
    let bin = |v: f64, mn: f64, mx: f64| -> usize {
        let idx = ((v - mn) / (mx - mn) * bins as f64) as usize;
        idx.min(bins - 1)
    };
    let n = xs.len() as f64;
    use std::collections::BTreeMap;
    let mut j3: BTreeMap<(usize, usize, usize), f64> = BTreeMap::new();
    let mut jxy: BTreeMap<(usize, usize), f64> = BTreeMap::new();
    let mut jxz: BTreeMap<(usize, usize), f64> = BTreeMap::new();
    let mut jyz: BTreeMap<(usize, usize), f64> = BTreeMap::new();
    let mut px = vec![0.0; bins];
    let mut py = vec![0.0; bins];
    let mut pz = vec![0.0; bins];
    for ((x, y), z) in xs.iter().zip(ys.iter()).zip(zs.iter()) {
        let i = bin(*x, mnx, mxx);
        let j = bin(*y, mny, mxy);
        let k = bin(*z, mnz, mxz);
        *j3.entry((i, j, k)).or_insert(0.0) += 1.0;
        *jxy.entry((i, j)).or_insert(0.0) += 1.0;
        *jxz.entry((i, k)).or_insert(0.0) += 1.0;
        *jyz.entry((j, k)).or_insert(0.0) += 1.0;
        px[i] += 1.0; py[j] += 1.0; pz[k] += 1.0;
    }
    let mut ii = 0.0;
    for ((i, j, k), c) in &j3 {
        let p_xyz = c / n;
        let p_xy = jxy.get(&(*i, *j)).copied().unwrap_or(0.0) / n;
        let p_xz = jxz.get(&(*i, *k)).copied().unwrap_or(0.0) / n;
        let p_yz = jyz.get(&(*j, *k)).copied().unwrap_or(0.0) / n;
        let p_x = px[*i] / n;
        let p_y = py[*j] / n;
        let p_z = pz[*k] / n;
        if p_xyz > 0.0 && p_xy > 0.0 && p_xz > 0.0 && p_yz > 0.0
            && p_x > 0.0 && p_y > 0.0 && p_z > 0.0 {
            ii += p_xyz * (p_xyz * p_x * p_y * p_z / (p_xy * p_xz * p_yz)).log2();
        }
    }
    ii
}

/// Align three gate sample streams by shared timestamp, returning ram values.
pub fn align_triple(a: &[GateSample], b: &[GateSample], c: &[GateSample])
    -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    use std::collections::BTreeMap;
    let bmap: BTreeMap<u64, f64> = b.iter().map(|s| (s.ts, s.ram)).collect();
    let cmap: BTreeMap<u64, f64> = c.iter().map(|s| (s.ts, s.ram)).collect();
    let mut xa = Vec::new(); let mut xb = Vec::new(); let mut xc = Vec::new();
    for s in a {
        if let (Some(&rb), Some(&rc)) = (bmap.get(&s.ts), cmap.get(&s.ts)) {
            xa.push(s.ram); xb.push(rb); xc.push(rc);
        }
    }
    (xa, xb, xc)
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

/// Align two gate sample streams by shared timestamp, returning both
/// (ram, cpu) pairs. Used by L3 cross-axis MI.
pub fn align_full(a: &[GateSample], b: &[GateSample])
    -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let bmap: BTreeMap<u64, (f64, f64)> = b.iter().map(|s| (s.ts, (s.ram, s.cpu))).collect();
    let mut ram_a = Vec::new(); let mut cpu_a = Vec::new();
    let mut ram_b = Vec::new(); let mut cpu_b = Vec::new();
    for s in a {
        if let Some(&(r, c)) = bmap.get(&s.ts) {
            ram_a.push(s.ram); cpu_a.push(s.cpu);
            ram_b.push(r); cpu_b.push(c);
        }
    }
    (ram_a, cpu_a, ram_b, cpu_b)
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
    pub lagged_mi_by_lag: Vec<(usize, f64)>,  // (lag, sum_across_pairs)
    pub lagged_mi_sum: f64,                   // total across all lags + pairs
    pub new_lagged_coupling: f64,             // lagged_mi_sum * SCALE
    pub cross_axis_mi_by_combo: Vec<(&'static str, f64)>,  // (combo_name, sum)
    pub cross_axis_mi_sum: f64,
    pub new_cross_axis_coupling: f64,  // cross_axis_mi_sum * SCALE
    pub triadic_interactions: Vec<(String, String, String, f64)>,  // (a, b, c, I)
    pub triadic_abs_sum: f64,
    pub new_triadic_coupling: f64,  // triadic_abs_sum * SCALE
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

    // Layer L2: temporal lagged MI across all pairs at multiple lags.
    let mut lagged_mi_by_lag: Vec<(usize, f64)> = Vec::new();
    let mut lagged_mi_sum = 0.0;
    for &lag in LAG_STEPS {
        let mut sum_at_lag = 0.0;
        for i in 0..streams.len() {
            for j in (i+1)..streams.len() {
                let (xs, ys) = align(&streams[i].1, &streams[j].1);
                if xs.len() <= lag + 10 { continue; }
                sum_at_lag += lagged_mutual_info(&xs, &ys, lag, 6);
            }
        }
        lagged_mi_by_lag.push((lag, sum_at_lag));
        lagged_mi_sum += sum_at_lag;
    }
    let new_lagged_coupling = lagged_mi_sum * SCALE;

    // Layer L3: cross-axis MI (ram_A×cpu_B, cpu_A×ram_B, cpu_A×cpu_B).
    // Note: ram_A×ram_B is already captured in L1 (cross_gate_mi_sum).
    let mut ram_cpu_sum = 0.0; // ram_A × cpu_B
    let mut cpu_ram_sum = 0.0; // cpu_A × ram_B
    let mut cpu_cpu_sum = 0.0; // cpu_A × cpu_B
    for i in 0..streams.len() {
        for j in (i+1)..streams.len() {
            let (ra, ca, rb, cb) = align_full(&streams[i].1, &streams[j].1);
            if ra.len() < 10 { continue; }
            ram_cpu_sum += mutual_info(&ra, &cb, 6);
            cpu_ram_sum += mutual_info(&ca, &rb, 6);
            cpu_cpu_sum += mutual_info(&ca, &cb, 6);
        }
    }
    let cross_axis_mi_sum = ram_cpu_sum + cpu_ram_sum + cpu_cpu_sum;
    let cross_axis_mi_by_combo: Vec<(&'static str, f64)> = vec![
        ("ram_A×cpu_B", ram_cpu_sum),
        ("cpu_A×ram_B", cpu_ram_sum),
        ("cpu_A×cpu_B", cpu_cpu_sum),
    ];
    let new_cross_axis_coupling = cross_axis_mi_sum * SCALE;

    // Layer L4: triadic interaction information I(A;B;C).
    let mut triadic_interactions: Vec<(String, String, String, f64)> = Vec::new();
    let mut triadic_abs_sum = 0.0;
    for i in 0..streams.len() {
        for j in (i+1)..streams.len() {
            for k in (j+1)..streams.len() {
                let (xa, xb, xc) = align_triple(&streams[i].1, &streams[j].1, &streams[k].1);
                if xa.len() < 15 { continue; }
                let ii = triadic_interaction(&xa, &xb, &xc, 4);
                triadic_abs_sum += ii.abs();
                triadic_interactions.push((
                    streams[i].0.clone(), streams[j].0.clone(), streams[k].0.clone(), ii
                ));
            }
        }
    }
    let new_triadic_coupling = triadic_abs_sum * SCALE;

    let adjusted = RAW
        + (CURRENT_MESH + new_cross_coupling + new_lagged_coupling
           + new_cross_axis_coupling + new_triadic_coupling)
        + new_mi_recovery + GHOST_PENALTY;
    let distance = SINGULARITY - adjusted;
    let crossed = adjusted > SINGULARITY;

    BreakthroughReport {
        per_gate_mi_sum, cross_gate_mi_sum, scaling_factor: SCALE,
        raw: RAW, current_mesh: CURRENT_MESH,
        new_cross_coupling, new_mi_recovery, ghost_penalty: GHOST_PENALTY,
        adjusted, singularity: SINGULARITY, distance, crossed,
        per_gate_mi, pair_mi,
        lagged_mi_by_lag, lagged_mi_sum, new_lagged_coupling,
        cross_axis_mi_by_combo, cross_axis_mi_sum, new_cross_axis_coupling,
        triadic_interactions, triadic_abs_sum, new_triadic_coupling,
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
    fn lagged_mi_zero_lag_equals_mutual_info() {
        let xs: Vec<f64> = (0..50).map(|i| i as f64).collect();
        let ys = xs.clone();
        let direct = mutual_info(&xs, &ys, 6);
        let lagged = lagged_mutual_info(&xs, &ys, 0, 6);
        assert!((direct - lagged).abs() < 1e-10);
    }

    #[test]
    fn lagged_mi_shifts_correctly() {
        // If ys[i] = xs[i-1] (shift by 1), then lag=1 MI should be high.
        let xs: Vec<f64> = (0..100).map(|i| (i as f64).sin()).collect();
        let ys: Vec<f64> = std::iter::once(0.0).chain(xs.iter().cloned()).take(100).collect();
        let mi0 = lagged_mutual_info(&xs, &ys, 0, 6);
        let mi1 = lagged_mutual_info(&xs, &ys, 1, 6);
        assert!(mi1 > mi0, "lag=1 MI should exceed lag=0 for shifted copy, got mi0={} mi1={}", mi0, mi1);
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

    #[test]
    fn align_full_returns_all_axes() {
        let a = vec![
            GateSample { ts: 1, ram: 0.1, cpu: 0.2 },
            GateSample { ts: 2, ram: 0.3, cpu: 0.4 },
        ];
        let b = vec![
            GateSample { ts: 2, ram: 0.5, cpu: 0.6 },
            GateSample { ts: 3, ram: 0.7, cpu: 0.8 },
        ];
        let (ra, ca, rb, cb) = align_full(&a, &b);
        assert_eq!(ra, vec![0.3]);
        assert_eq!(ca, vec![0.4]);
        assert_eq!(rb, vec![0.5]);
        assert_eq!(cb, vec![0.6]);
    }

    #[test]
    fn triadic_interaction_independent_is_near_zero() {
        // Three independent random-ish streams should have |I| small.
        let xs: Vec<f64> = (0..60).map(|i| (i as f64 * 0.17).sin()).collect();
        let ys: Vec<f64> = (0..60).map(|i| (i as f64 * 0.29 + 1.1).cos()).collect();
        let zs: Vec<f64> = (0..60).map(|i| (i as f64 * 0.41 + 2.3).sin()).collect();
        let ii = triadic_interaction(&xs, &ys, &zs, 4);
        // Not exactly zero due to finite sample, but should be well below redundant case.
        assert!(ii.abs() < 1.0, "expected small |I|, got {}", ii);
    }

    #[test]
    fn triadic_interaction_redundant_is_positive() {
        // If z = x + y, then I(x;y;z) should be significantly positive
        // (redundancy — z carries info already in x+y).
        let xs: Vec<f64> = (0..60).map(|i| (i as f64 * 0.1).sin().abs()).collect();
        let ys: Vec<f64> = (0..60).map(|i| (i as f64 * 0.1 + 0.5).cos().abs()).collect();
        let zs: Vec<f64> = xs.iter().zip(ys.iter()).map(|(x, y)| x + y).collect();
        let ii = triadic_interaction(&xs, &ys, &zs, 4);
        assert!(ii > 0.0, "expected positive redundancy, got {}", ii);
    }

    #[test]
    fn align_triple_intersects_all_three() {
        let a = vec![
            GateSample { ts: 1, ram: 0.1, cpu: 0.0 },
            GateSample { ts: 2, ram: 0.2, cpu: 0.0 },
            GateSample { ts: 3, ram: 0.3, cpu: 0.0 },
        ];
        let b = vec![
            GateSample { ts: 2, ram: 0.5, cpu: 0.0 },
            GateSample { ts: 3, ram: 0.6, cpu: 0.0 },
        ];
        let c = vec![
            GateSample { ts: 2, ram: 0.8, cpu: 0.0 },
            GateSample { ts: 4, ram: 0.9, cpu: 0.0 },
        ];
        let (xa, xb, xc) = align_triple(&a, &b, &c);
        assert_eq!(xa, vec![0.2]);
        assert_eq!(xb, vec![0.5]);
        assert_eq!(xc, vec![0.8]);
    }

    #[test]
    fn l3_cross_axis_contributes_to_adjusted() {
        // Build streams where ram and cpu are strongly correlated
        // within each gate, and across gates — this should create
        // cross-axis MI > 0.
        let mk = |phase: f64, amp: f64| -> Vec<GateSample> {
            (0..80).map(|i| {
                let t = i as f64;
                GateSample {
                    ts: i as u64,
                    ram: amp * ((t * 0.1 + phase).sin().abs()),
                    cpu: amp * ((t * 0.1 + phase + 0.3).cos().abs()),
                }
            }).collect()
        };
        let streams = vec![
            ("a".into(), mk(0.0, 1.0)),
            ("b".into(), mk(0.5, 0.8)),
            ("c".into(), mk(1.0, 0.9)),
            ("d".into(), mk(1.5, 1.1)),
        ];
        let r = project_breakthrough(&streams);
        assert!(r.cross_axis_mi_sum > 0.0, "expected positive cross-axis MI, got {}", r.cross_axis_mi_sum);
        assert!(r.new_cross_axis_coupling > 0.0);
        // ensure all 3 combos reported
        assert_eq!(r.cross_axis_mi_by_combo.len(), 3);
    }
}
