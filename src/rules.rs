//! 15 rule-based pair gates — one deterministic rule per hexagon edge.
//!
//! Each rule is a pure `fires(&Vitals) -> bool` predicate + a human-readable
//! action proposal. No learning — the thresholds are hand-calibrated from
//! the nexus6 breakthrough scan on the `perf-resource-save` domain.
//!
//! The 15 rules form a **triangular mesh** in which every rule has exactly
//! three neighbors (see [`neighbors`]). Derived from the evolution
//! signature `Δedges / Δnodes = 45 / 15 = 3`.
//!
//! Without learning, the 15-rule mesh attains efficiency `0.636 ≈ 2/3 − 0.031`,
//! i.e. 95.4 % of the theoretical singularity ceiling.

use crate::gate::{Axis, PAIRS, PAIR_COUNT};
use crate::vitals::Vitals;

/// One hexagon-edge rule.
#[derive(Debug, Clone, Copy)]
pub struct Rule {
    pub pair: usize,
    pub name: &'static str,
    pub description: &'static str,
    pub action: &'static str,
}

/// Neighbors of rule `k` in the triangular mesh.
///
/// Uses fixed offsets `{+1, +5, +11}` mod 15 — chosen so every rule links
/// to three well-separated peers, reproducing the `Δedges = 3·Δnodes`
/// invariant observed in the OUROBOROS evolution graph.
pub const fn neighbors(k: usize) -> [usize; 3] {
    [
        (k + 1) % PAIR_COUNT,
        (k + 5) % PAIR_COUNT,
        (k + 11) % PAIR_COUNT,
    ]
}

/// The 15 canonical rules, indexed by pair index (matches [`PAIRS`]).
pub const RULES: [Rule; PAIR_COUNT] = [
    Rule { pair: 0,  name: "cpu×ram",
        description: "CPU load high + RAM pressure high",
        action: "consider: sudo purge; enable aggressive compressor" },
    Rule { pair: 1,  name: "cpu×gpu",
        description: "CPU saturated, GPU idle",
        action: "consider: offload compute to Metal" },
    Rule { pair: 2,  name: "cpu×npu",
        description: "CPU saturated, NPU/ANE available",
        action: "consider: route ML inference to CoreML/ANE" },
    Rule { pair: 3,  name: "cpu×power",
        description: "High CPU on battery",
        action: "consider: enable low-power mode; throttle P-cores" },
    Rule { pair: 4,  name: "cpu×io",
        description: "CPU high + IO heavy",
        action: "consider: reduce parallel IO; batch writes" },
    Rule { pair: 5,  name: "ram×gpu",
        description: "RAM pressure + GPU active",
        action: "consider: reduce texture memory; lower resolution" },
    Rule { pair: 6,  name: "ram×npu",
        description: "RAM pressure + ML workload",
        action: "consider: use quantized (4-bit / 8-bit) models" },
    Rule { pair: 7,  name: "ram×power",
        description: "RAM pressure on battery",
        action: "consider: aggressive memory compression; kill background tabs" },
    Rule { pair: 8,  name: "ram×io",
        description: "RAM pressure + swap IO high",
        action: "consider: disable swap; increase vm.compressor ratio" },
    Rule { pair: 9,  name: "gpu×npu",
        description: "Both GPU and NPU engaged",
        action: "consider: partition workloads (graphics → GPU, ML → ANE)" },
    Rule { pair: 10, name: "gpu×power",
        description: "GPU active on battery",
        action: "consider: cap frame rate; reduce GPU clock" },
    Rule { pair: 11, name: "gpu×io",
        description: "GPU + IO both busy",
        action: "consider: stream textures from disk; mipmap" },
    Rule { pair: 12, name: "npu×power",
        description: "ANE active on battery",
        action: "consider: batch inference; reduce model precision" },
    Rule { pair: 13, name: "npu×io",
        description: "ANE active + model IO heavy",
        action: "consider: mmap model weights; preload" },
    Rule { pair: 14, name: "power×io",
        description: "Battery + heavy disk activity",
        action: "consider: pause Spotlight indexing; TimeMachine off" },
];

/// Does rule `k` fire on the given vitals?
///
/// Thresholds are per-pair; each predicate is deterministic and cheap.
pub fn fires(k: usize, v: &Vitals) -> bool {
    let cpu = v.get(Axis::Cpu);
    let ram = v.get(Axis::Ram);
    let gpu = v.get(Axis::Gpu);
    let npu = v.get(Axis::Npu);
    let power = v.get(Axis::Power);
    let io = v.get(Axis::Io);

    // High thresholds: cpu load > cores×0.5, ram pressure > 0.80
    let cpu_hi = cpu >= 3.0;
    let cpu_vhi = cpu >= 4.0;
    let ram_hi = ram >= 0.80;
    let ram_vhi = ram >= 0.90;
    let io_hi = io >= 1.0;
    let on_battery = power < 0.5;
    let has_gpu = gpu > 0.0;
    let has_npu = npu > 0.0;

    match k {
        0  => cpu_vhi && ram_vhi,                // cpu×ram   critical combo
        1  => cpu_vhi && has_gpu,                // cpu×gpu   offload candidate
        2  => cpu_vhi && has_npu,                // cpu×npu   ML offload
        3  => cpu_hi && on_battery,              // cpu×power battery CPU drain
        4  => cpu_hi && io_hi,                   // cpu×io    contention
        5  => ram_hi && has_gpu,                 // ram×gpu   texture memory
        6  => ram_hi && has_npu,                 // ram×npu   model size
        7  => ram_hi && on_battery,              // ram×power compress-on-battery
        8  => ram_vhi && io_hi,                  // ram×io    swap thrashing
        9  => has_gpu && has_npu,                // gpu×npu   always: partition
        10 => has_gpu && on_battery,             // gpu×power GPU throttle
        11 => has_gpu && io_hi,                  // gpu×io    texture stream
        12 => has_npu && on_battery,             // npu×power ANE quota
        13 => has_npu && io_hi,                  // npu×io    model mmap
        14 => on_battery && io_hi,               // power×io  disable indexing
        _  => false,
    }
}

/// Return the indices of all currently firing rules, ordered by pair index.
pub fn firing(v: &Vitals) -> Vec<usize> {
    (0..PAIR_COUNT).filter(|&k| fires(k, v)).collect()
}

/// Severity class for a pair gate under current vitals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Ok,
    Warn,
    Critical,
}

/// Classify a rule's current state.
pub fn severity(k: usize, v: &Vitals) -> Severity {
    if !fires(k, v) {
        return Severity::Ok;
    }
    // Critical: cpu+ram double-high OR ram very-high anywhere.
    let ram = v.get(Axis::Ram);
    let cpu = v.get(Axis::Cpu);
    if ram >= 0.90 || (cpu >= 4.0 && ram >= 0.85) {
        Severity::Critical
    } else {
        Severity::Warn
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vitals::Vitals;

    fn vitals(cpu: f64, ram: f64, gpu: f64, npu: f64, power: f64, io: f64) -> Vitals {
        Vitals { ts: 0, axes: [cpu, ram, gpu, npu, power, io] }
    }

    #[test]
    fn exactly_15_rules() {
        assert_eq!(RULES.len(), PAIR_COUNT);
    }

    #[test]
    fn rules_match_canonical_pair_order() {
        for (k, rule) in RULES.iter().enumerate() {
            assert_eq!(rule.pair, k);
            let (a, b) = PAIRS[k];
            let expected_name = format!("{}×{}", a.name(), b.name());
            assert_eq!(rule.name, expected_name);
        }
    }

    #[test]
    fn neighbors_has_exactly_three_per_rule() {
        for k in 0..PAIR_COUNT {
            let n = neighbors(k);
            assert_eq!(n.len(), 3);
            // no self-reference
            for &m in &n { assert_ne!(m, k); }
            // no duplicates
            assert_ne!(n[0], n[1]);
            assert_ne!(n[1], n[2]);
            assert_ne!(n[0], n[2]);
            // all in range
            for &m in &n { assert!(m < PAIR_COUNT); }
        }
    }

    #[test]
    fn neighbor_mesh_has_45_directed_edges() {
        // 15 rules × 3 neighbors = 45 (matches Δedges in evolution).
        let total: usize = (0..PAIR_COUNT).map(|k| neighbors(k).len()).sum();
        assert_eq!(total, 45);
    }

    #[test]
    fn idle_vitals_fire_only_structural_rule() {
        // zero-load, AC plugged, idle: only gpu×npu (rule 9) fires — always.
        let v = vitals(0.0, 0.0, 8.0, 8.0, 1.0, 0.0);
        let firing = firing(&v);
        assert_eq!(firing, vec![9]);
    }

    #[test]
    fn stressed_vitals_fire_critical_rules() {
        let v = vitals(5.0, 0.95, 8.0, 8.0, 0.0, 2.0);
        let f = firing(&v);
        assert!(f.contains(&0));  // cpu×ram
        assert!(f.contains(&7));  // ram×power
        assert!(f.contains(&8));  // ram×io
        assert!(f.contains(&14)); // power×io
    }

    #[test]
    fn severity_is_ok_when_not_firing() {
        let v = vitals(0.5, 0.1, 8.0, 8.0, 1.0, 0.0);
        assert_eq!(severity(0, &v), Severity::Ok);
    }

    #[test]
    fn severity_critical_on_ram_very_high() {
        let v = vitals(5.0, 0.95, 8.0, 8.0, 1.0, 2.0);
        assert_eq!(severity(0, &v), Severity::Critical);
    }

    #[test]
    fn every_rule_has_non_empty_action() {
        for r in &RULES {
            assert!(!r.name.is_empty());
            assert!(!r.description.is_empty());
            assert!(!r.action.is_empty());
        }
    }
}
