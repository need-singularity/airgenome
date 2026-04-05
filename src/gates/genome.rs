//! GateGenome — 60-byte per-source genome record.
//!
//! Layout (60 bytes, little-endian):
//!   offset  size  field
//!     0     24    6-axis values (6 × f32)
//!    24      2    15-pair firing bits (u16, packed)
//!    26      2    padding
//!    28      4    timestamp (u32 unix seconds)
//!    32     12    interface-specific counters (3 × f32: procs, rss_mb, cpu_pct)
//!    44     16    moving stats (min/max/mean/stddev across 6 axes as 4 × f32)
//!    60     total

use serde::{Deserialize, Serialize};

pub const GATE_GENOME_BYTES: usize = 60;

/// Index of the process-count counter in `GateGenome::counters`.
pub const COUNTER_PROCS: usize = 0;
/// Index of the RSS (MB) counter in `GateGenome::counters`.
pub const COUNTER_RSS_MB: usize = 1;
/// Index of the CPU percent counter in `GateGenome::counters`.
pub const COUNTER_CPU_PCT: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GateGenome {
    pub axes: [f32; 6],
    pub firing_bits: u16,
    pub ts: u32,
    /// Interface-specific counters: [procs, rss_mb, cpu_pct].
    /// Use COUNTER_PROCS / COUNTER_RSS_MB / COUNTER_CPU_PCT indexes.
    pub counters: [f32; 3],
    pub stats: [f32; 4],
}

impl GateGenome {
    pub const fn zeroed() -> Self {
        Self {
            axes: [0.0; 6],
            firing_bits: 0,
            ts: 0,
            counters: [0.0; 3],
            stats: [0.0; 4],
        }
    }

    /// Serialize to exactly 60 bytes (little-endian).
    pub fn to_bytes(&self) -> [u8; GATE_GENOME_BYTES] {
        let mut out = [0u8; GATE_GENOME_BYTES];
        for i in 0..6 {
            out[i*4..(i+1)*4].copy_from_slice(&self.axes[i].to_le_bytes());
        }
        out[24..26].copy_from_slice(&self.firing_bits.to_le_bytes());
        // bytes 26..28 are padding (already zero)
        out[28..32].copy_from_slice(&self.ts.to_le_bytes());
        for i in 0..3 {
            out[32 + i*4..32 + (i+1)*4].copy_from_slice(&self.counters[i].to_le_bytes());
        }
        for i in 0..4 {
            out[44 + i*4..44 + (i+1)*4].copy_from_slice(&self.stats[i].to_le_bytes());
        }
        out
    }

    /// Deserialize from exactly 60 bytes.
    pub fn from_bytes(b: &[u8; GATE_GENOME_BYTES]) -> Self {
        let mut axes = [0f32; 6];
        for i in 0..6 {
            axes[i] = f32::from_le_bytes(b[i*4..(i+1)*4].try_into().unwrap());
        }
        let firing_bits = u16::from_le_bytes(b[24..26].try_into().unwrap());
        let ts = u32::from_le_bytes(b[28..32].try_into().unwrap());
        let mut counters = [0f32; 3];
        for i in 0..3 {
            counters[i] = f32::from_le_bytes(b[32 + i*4..32 + (i+1)*4].try_into().unwrap());
        }
        let mut stats = [0f32; 4];
        for i in 0..4 {
            stats[i] = f32::from_le_bytes(b[44 + i*4..44 + (i+1)*4].try_into().unwrap());
        }
        Self { axes, firing_bits, ts, counters, stats }
    }

    /// Populate stats (min/max/mean/stddev) from the 6 axes.
    pub fn populate_stats(&mut self) {
        let mn = self.axes.iter().cloned().fold(f32::INFINITY, f32::min);
        let mx = self.axes.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let mean = self.axes.iter().sum::<f32>() / 6.0;
        let var = self.axes.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / 6.0;
        self.stats = [mn, mx, mean, var.sqrt()];
    }

    /// Test whether pair `k` (0..15) is firing.
    pub fn fires(&self, k: usize) -> bool {
        k < 15 && (self.firing_bits & (1 << k)) != 0
    }

    /// Set firing bit for pair `k` (0..15).
    pub fn set_firing(&mut self, k: usize, on: bool) {
        if k >= 15 { return; }
        if on { self.firing_bits |= 1 << k; }
        else  { self.firing_bits &= !(1 << k); }
    }

    /// Count how many pairs are firing.
    pub fn firing_count(&self) -> u32 {
        self.firing_bits.count_ones()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_is_60_bytes() {
        let g = GateGenome::zeroed();
        assert_eq!(g.to_bytes().len(), 60);
    }

    #[test]
    fn round_trip_preserves_all_fields() {
        let mut g = GateGenome::zeroed();
        g.axes = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6];
        g.firing_bits = 0b101_0101_0101_0101;
        g.ts = 1775397756;
        g.counters = [42.0, 1024.5, 85.0];
        g.populate_stats();
        let bytes = g.to_bytes();
        let back = GateGenome::from_bytes(&bytes);
        assert_eq!(back, g);
    }

    #[test]
    fn firing_bit_set_get() {
        let mut g = GateGenome::zeroed();
        assert!(!g.fires(3));
        g.set_firing(3, true);
        assert!(g.fires(3));
        assert_eq!(g.firing_count(), 1);
        g.set_firing(3, false);
        assert!(!g.fires(3));
        g.set_firing(14, true);
        g.set_firing(0, true);
        assert_eq!(g.firing_count(), 2);
    }

    #[test]
    fn firing_bit_out_of_range_noop() {
        let mut g = GateGenome::zeroed();
        g.set_firing(99, true);
        assert_eq!(g.firing_bits, 0);
        assert!(!g.fires(99));
    }

    #[test]
    fn populate_stats_computes_correctly() {
        let mut g = GateGenome::zeroed();
        g.axes = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        g.populate_stats();
        assert_eq!(g.stats[0], 1.0); // min
        assert_eq!(g.stats[1], 6.0); // max
        assert!((g.stats[2] - 3.5).abs() < 1e-5); // mean
        assert!((g.stats[3] - 1.7078).abs() < 1e-3); // stddev (population, sqrt(35/12))
    }
}
