//! Signature — 6-axis vectors + distance metrics + workload fingerprints.
//!
//! Implements the primitives the CLAUDE.md mission needs:
//! - per-source/per-moment 6-axis vector representation
//! - distance (Euclidean + cosine) between signatures
//! - built-in workload fingerprint library
//! - nearest-fingerprint matching

use serde::{Deserialize, Serialize};

/// A 6-dimensional signature: `[cpu, ram, gpu, npu, power, io]`.
/// Values are in `[0, ∞)` but most are expected in `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Signature {
    pub axes: [f64; 6],
}

impl Signature {
    pub const fn new(axes: [f64; 6]) -> Self { Self { axes } }
    pub const fn zero() -> Self { Self { axes: [0.0; 6] } }

    /// Euclidean distance between two signatures.
    pub fn euclidean(&self, other: &Signature) -> f64 {
        let mut sum = 0.0;
        for i in 0..6 {
            let d = self.axes[i] - other.axes[i];
            sum += d * d;
        }
        sum.sqrt()
    }

    /// Cosine similarity in `[-1, 1]` (1 = identical direction, 0 = orthogonal).
    /// Returns 0 if either vector has zero magnitude.
    pub fn cosine(&self, other: &Signature) -> f64 {
        let mut dot = 0.0;
        let mut na = 0.0;
        let mut nb = 0.0;
        for i in 0..6 {
            dot += self.axes[i] * other.axes[i];
            na += self.axes[i] * self.axes[i];
            nb += other.axes[i] * other.axes[i];
        }
        if na <= 0.0 || nb <= 0.0 { return 0.0; }
        dot / (na.sqrt() * nb.sqrt())
    }

    /// L-infinity norm — the single largest axis magnitude.
    pub fn l_inf(&self) -> f64 {
        let mut max = 0.0f64;
        for &v in &self.axes { if v.abs() > max { max = v.abs(); } }
        max
    }
}

/// A named fingerprint from the built-in library.
#[derive(Debug, Clone, Copy)]
pub struct Fingerprint {
    pub name: &'static str,
    pub description: &'static str,
    pub signature: Signature,
}

/// Built-in workload fingerprints. Values are indicative, tuned to match
/// the `airgenome simulate` scenarios plus common workloads.
pub const FINGERPRINTS: &[Fingerprint] = &[
    Fingerprint {
        name: "idle",
        description: "light load, plugged in, RAM slack",
        signature: Signature::new([0.3, 0.15, 8.0, 8.0, 1.0, 0.5]),
    },
    Fingerprint {
        name: "compile",
        description: "CPU-bound build (rustc/cargo/xcodebuild)",
        signature: Signature::new([7.0, 0.45, 8.0, 8.0, 1.0, 2.0]),
    },
    Fingerprint {
        name: "browse",
        description: "browser-dominated, ram elevated",
        signature: Signature::new([2.0, 0.70, 8.0, 8.0, 1.0, 1.2]),
    },
    Fingerprint {
        name: "ml-inference",
        description: "GPU/NPU active, RAM tight",
        signature: Signature::new([4.0, 0.88, 8.0, 8.0, 1.0, 1.2]),
    },
    Fingerprint {
        name: "video-encode",
        description: "sustained high CPU + GPU, IO heavy",
        signature: Signature::new([7.5, 0.55, 8.0, 8.0, 1.0, 3.0]),
    },
    Fingerprint {
        name: "battery-idle",
        description: "unplugged, low load, low RAM",
        signature: Signature::new([0.5, 0.20, 8.0, 8.0, 0.0, 0.3]),
    },
    Fingerprint {
        name: "ram-thrash",
        description: "RAM pressure critical, swap active",
        signature: Signature::new([4.0, 0.95, 8.0, 8.0, 1.0, 3.5]),
    },
];

/// Return the nearest fingerprint (Euclidean distance) to `sig`.
pub fn nearest(sig: &Signature) -> (&'static Fingerprint, f64) {
    let mut best = &FINGERPRINTS[0];
    let mut best_d = sig.euclidean(&best.signature);
    for fp in &FINGERPRINTS[1..] {
        let d = sig.euclidean(&fp.signature);
        if d < best_d { best_d = d; best = fp; }
    }
    (best, best_d)
}

/// Look up a fingerprint by name.
pub fn by_name(name: &str) -> Option<&'static Fingerprint> {
    FINGERPRINTS.iter().find(|fp| fp.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn euclidean_identity_is_zero() {
        let s = Signature::new([1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert!((s.euclidean(&s) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn euclidean_pythagorean() {
        let a = Signature::new([3.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let b = Signature::new([0.0, 4.0, 0.0, 0.0, 0.0, 0.0]);
        assert!((a.euclidean(&b) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn cosine_identical_is_one() {
        let s = Signature::new([1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert!((s.cosine(&s) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn cosine_orthogonal_is_zero() {
        let a = Signature::new([1.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let b = Signature::new([0.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
        assert!((a.cosine(&b) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn cosine_zero_vector_returns_zero() {
        let z = Signature::zero();
        let s = Signature::new([1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(z.cosine(&s), 0.0);
    }

    #[test]
    fn l_inf_picks_max_axis() {
        let s = Signature::new([1.0, -5.0, 2.0, 3.0, 0.0, 4.0]);
        assert!((s.l_inf() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn fingerprints_library_non_empty() {
        assert!(FINGERPRINTS.len() >= 5);
        for fp in FINGERPRINTS {
            assert!(!fp.name.is_empty());
            assert!(!fp.description.is_empty());
        }
    }

    #[test]
    fn by_name_finds_fingerprints() {
        assert!(by_name("idle").is_some());
        assert!(by_name("compile").is_some());
        assert!(by_name("nonexistent").is_none());
    }

    #[test]
    fn nearest_matches_own_fingerprint() {
        for fp in FINGERPRINTS {
            let (best, d) = nearest(&fp.signature);
            assert_eq!(best.name, fp.name);
            assert!(d < 1e-9, "expected d=0, got {}", d);
        }
    }

    #[test]
    fn nearest_returns_closest() {
        // A vector close to idle but offset.
        let close_to_idle = Signature::new([0.31, 0.16, 8.0, 8.0, 1.0, 0.6]);
        let (best, _) = nearest(&close_to_idle);
        assert_eq!(best.name, "idle");
    }
}
