//! 6-axis Resource Hexagon + 15 Pair Gates.
//!
//! Each axis represents one MacBook resource dimension; every unordered pair
//! `(a, b)` defines a gate whose `u32` state encodes the learned interaction
//! policy. The full genome = `[u32; 15] = 60 bytes`.

use serde::{Deserialize, Serialize};

/// The six resource axes of the MacBook hexagon.
///
/// Ordering is fixed: enumeration defines pair indices via `pair_index(a, b)`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Axis {
    Cpu = 0,
    Ram = 1,
    Gpu = 2,
    Npu = 3,
    Power = 4,
    Io = 5,
}

/// Number of resource axes (n = 6).
pub const AXIS_COUNT: usize = 6;
/// Number of unordered pairs: C(6, 2) = 15.
pub const PAIR_COUNT: usize = 15;
/// Genome size in bytes: 15 pairs × 4 bytes.
pub const GENOME_BYTES: usize = PAIR_COUNT * 4;

impl Axis {
    pub const ALL: [Axis; AXIS_COUNT] = [
        Axis::Cpu, Axis::Ram, Axis::Gpu, Axis::Npu, Axis::Power, Axis::Io,
    ];

    pub const fn index(self) -> usize { self as usize }

    pub const fn name(self) -> &'static str {
        match self {
            Axis::Cpu => "cpu",
            Axis::Ram => "ram",
            Axis::Gpu => "gpu",
            Axis::Npu => "npu",
            Axis::Power => "power",
            Axis::Io => "io",
        }
    }
}

/// Compute the canonical pair index for axes `(a, b)` using the strictly
/// upper-triangular convention (`a.index() < b.index()`).
///
/// Returns `None` if `a == b`.
pub fn pair_index(a: Axis, b: Axis) -> Option<usize> {
    let (i, j) = {
        let (x, y) = (a.index(), b.index());
        if x == y { return None; }
        if x < y { (x, y) } else { (y, x) }
    };
    // Upper-triangular linearization: sum of row widths + column offset.
    // row i has (AXIS_COUNT - 1 - i) entries.
    let base: usize = (0..i).map(|k| AXIS_COUNT - 1 - k).sum();
    Some(base + (j - i - 1))
}

/// The 15 canonical unordered pairs in the hexagon, in `pair_index` order.
pub const PAIRS: [(Axis, Axis); PAIR_COUNT] = [
    (Axis::Cpu, Axis::Ram),   (Axis::Cpu, Axis::Gpu),   (Axis::Cpu, Axis::Npu),
    (Axis::Cpu, Axis::Power), (Axis::Cpu, Axis::Io),
    (Axis::Ram, Axis::Gpu),   (Axis::Ram, Axis::Npu),   (Axis::Ram, Axis::Power),
    (Axis::Ram, Axis::Io),
    (Axis::Gpu, Axis::Npu),   (Axis::Gpu, Axis::Power), (Axis::Gpu, Axis::Io),
    (Axis::Npu, Axis::Power), (Axis::Npu, Axis::Io),
    (Axis::Power, Axis::Io),
];

/// Pair-gate state (4 bytes) — learned policy for an axis interaction.
///
/// Bit layout (opaque for now — implant's learning loop decides semantics):
/// ```text
///   bits  0..8   : engagement level (0..=255)
///   bits  8..16  : stability counter
///   bits 16..24  : surprise / anomaly counter
///   bits 24..32  : reserved / generation
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairGate(pub u32);

impl PairGate {
    pub const DISENGAGED: PairGate = PairGate(0);

    pub const fn engagement(self) -> u8 { (self.0 & 0xFF) as u8 }
    pub const fn engaged(self) -> bool { self.engagement() > 0 }
}

impl Default for PairGate {
    fn default() -> Self { PairGate::DISENGAGED }
}

/// The 60-byte genome: learned state for all 15 pair gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Genome {
    pub pairs: [PairGate; PAIR_COUNT],
}

impl Genome {
    pub const fn empty() -> Self {
        Self { pairs: [PairGate::DISENGAGED; PAIR_COUNT] }
    }

    /// Serialize to exactly `GENOME_BYTES` (60) bytes, little-endian.
    pub fn to_bytes(&self) -> [u8; GENOME_BYTES] {
        let mut out = [0u8; GENOME_BYTES];
        for (i, gate) in self.pairs.iter().enumerate() {
            let b = gate.0.to_le_bytes();
            out[i * 4..i * 4 + 4].copy_from_slice(&b);
        }
        out
    }

    /// Deserialize from exactly `GENOME_BYTES` bytes.
    pub fn from_bytes(bytes: &[u8; GENOME_BYTES]) -> Self {
        let mut g = Genome::empty();
        for i in 0..PAIR_COUNT {
            let mut b = [0u8; 4];
            b.copy_from_slice(&bytes[i * 4..i * 4 + 4]);
            g.pairs[i] = PairGate(u32::from_le_bytes(b));
        }
        g
    }
}

impl Default for Genome {
    fn default() -> Self { Genome::empty() }
}

/// The resource gate: hexagon + genome + singularity predicate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGate {
    pub genome: Genome,
}

impl ResourceGate {
    pub fn new() -> Self { Self { genome: Genome::empty() } }

    pub fn with_genome(genome: Genome) -> Self { Self { genome } }

    /// Count pair gates with non-zero engagement.
    pub fn active_pairs(&self) -> usize {
        self.genome.pairs.iter().filter(|p| p.engaged()).count()
    }

    /// Average degree: each active pair contributes 2 to the total degree
    /// (once per endpoint). With all 15 pairs active this equals
    /// `2 × 15 / 6 = 5` interaction-degree, plus 1 self-loop per axis → 6.
    pub fn avg_degree(&self) -> f64 {
        let interaction = 2.0 * self.active_pairs() as f64 / AXIS_COUNT as f64;
        interaction + 1.0 // self-loop accounts for the +1 (n=6 closure)
    }

    /// Singularity reached when all 15 pairs engage AND the efficiency score
    /// settles at the 2/3 meta-fixed-point AND avg_degree hits 6.
    pub fn singularity_reached(&self, efficiency: f64) -> bool {
        self.active_pairs() == PAIR_COUNT
            && (efficiency - 2.0 / 3.0).abs() < 0.01
            && (self.avg_degree() - AXIS_COUNT as f64).abs() < f64::EPSILON
    }
}

impl Default for ResourceGate {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_count_is_six() {
        assert_eq!(AXIS_COUNT, 6);
        assert_eq!(Axis::ALL.len(), 6);
    }

    #[test]
    fn pair_count_is_fifteen() {
        assert_eq!(PAIR_COUNT, 15);
        assert_eq!(PAIRS.len(), 15);
        // C(6, 2) = 15
        assert_eq!(PAIR_COUNT, (AXIS_COUNT * (AXIS_COUNT - 1)) / 2);
    }

    #[test]
    fn genome_is_sixty_bytes() {
        assert_eq!(GENOME_BYTES, 60);
        let g = Genome::empty();
        assert_eq!(g.to_bytes().len(), 60);
    }

    #[test]
    fn pair_index_is_canonical_and_unique() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for &(a, b) in &PAIRS {
            let idx = pair_index(a, b).unwrap();
            assert!(seen.insert(idx), "duplicate index {idx}");
        }
        assert_eq!(seen.len(), PAIR_COUNT);
        // indices form 0..15
        for k in 0..PAIR_COUNT { assert!(seen.contains(&k)); }
    }

    #[test]
    fn pair_index_symmetric() {
        for &(a, b) in &PAIRS {
            assert_eq!(pair_index(a, b), pair_index(b, a));
        }
        assert_eq!(pair_index(Axis::Cpu, Axis::Cpu), None);
    }

    #[test]
    fn pairs_matches_pair_index() {
        for (expected, &(a, b)) in PAIRS.iter().enumerate() {
            assert_eq!(pair_index(a, b), Some(expected));
        }
    }

    #[test]
    fn genome_roundtrip() {
        let mut g = Genome::empty();
        for (i, gate) in g.pairs.iter_mut().enumerate() {
            *gate = PairGate((i as u32) * 0x01020304 + 1);
        }
        let bytes = g.to_bytes();
        let decoded = Genome::from_bytes(&bytes);
        assert_eq!(g, decoded);
    }

    #[test]
    fn fully_engaged_avg_degree_is_six() {
        let mut g = Genome::empty();
        for gate in g.pairs.iter_mut() { *gate = PairGate(1); }
        let rg = ResourceGate::with_genome(g);
        assert_eq!(rg.active_pairs(), PAIR_COUNT);
        assert!((rg.avg_degree() - 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn singularity_requires_full_engagement_and_fixed_point() {
        let empty = ResourceGate::new();
        assert!(!empty.singularity_reached(2.0 / 3.0));

        let mut g = Genome::empty();
        for gate in g.pairs.iter_mut() { *gate = PairGate(0xFF); }
        let full = ResourceGate::with_genome(g);
        assert!(full.singularity_reached(2.0 / 3.0));
        assert!(!full.singularity_reached(0.5)); // wrong efficiency
    }
}
