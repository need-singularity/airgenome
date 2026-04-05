//! Profiles — hand-crafted 60-byte genomes for common scenarios.
//!
//! A profile is a [`Genome`] whose 15 pair gates are pre-engaged for a
//! specific usage mode (battery, performance, ML, etc.). No learning is
//! required to use profiles: pick one, apply it, done.

use crate::gate::{Genome, PairGate, PAIR_COUNT};

/// A named built-in profile.
#[derive(Debug, Clone, Copy)]
pub struct Profile {
    pub name: &'static str,
    pub description: &'static str,
    /// Indices (0..15) of the pair gates engaged under this profile.
    pub engaged_pairs: &'static [usize],
}

impl Profile {
    /// Build the 60-byte genome for this profile.
    ///
    /// Engaged pairs get `engagement = 0xFF`; others remain disengaged.
    pub fn genome(&self) -> Genome {
        let mut g = Genome::empty();
        for &k in self.engaged_pairs {
            if k < PAIR_COUNT {
                g.pairs[k] = PairGate(0xFF);
            }
        }
        g
    }

    /// How many pairs this profile engages.
    pub fn active_count(&self) -> usize { self.engaged_pairs.len() }
}

/// All five built-in profiles, in canonical order.
///
/// The pair-index cheat sheet (see `PAIRS`):
/// ```text
/// 0: cpu×ram   1: cpu×gpu   2: cpu×npu   3: cpu×power 4: cpu×io
/// 5: ram×gpu   6: ram×npu   7: ram×power 8: ram×io
/// 9: gpu×npu   10: gpu×power 11: gpu×io
/// 12: npu×power 13: npu×io
/// 14: power×io
/// ```
pub const PROFILES: [Profile; 5] = [
    Profile {
        name: "balanced",
        description: "default — all pairs engaged (full hexagon)",
        engaged_pairs: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
    },
    Profile {
        name: "battery-save",
        description: "battery mode — throttle and defer when unplugged",
        engaged_pairs: &[3, 7, 10, 12, 14], // all *×power + power×io
    },
    Profile {
        name: "performance",
        description: "AC + max throughput — heavy compute, no throttling",
        engaged_pairs: &[1, 2, 9], // GPU/NPU offload gates only
    },
    Profile {
        name: "dev-work",
        description: "compile / build — CPU, RAM, IO focused",
        engaged_pairs: &[0, 4, 8], // cpu×ram, cpu×io, ram×io
    },
    Profile {
        name: "ml-inference",
        description: "local LLM / ANE inference — GPU/NPU focused",
        engaged_pairs: &[1, 2, 6, 9, 13], // *×gpu/npu and ram×npu
    },
];

/// Look up a profile by name.
pub fn by_name(name: &str) -> Option<&'static Profile> {
    PROFILES.iter().find(|p| p.name == name)
}

/// Summary of all profile names.
pub fn names() -> Vec<&'static str> {
    PROFILES.iter().map(|p| p.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_profiles_exist() {
        assert_eq!(PROFILES.len(), 5);
    }

    #[test]
    fn names_are_unique() {
        use std::collections::HashSet;
        let set: HashSet<_> = PROFILES.iter().map(|p| p.name).collect();
        assert_eq!(set.len(), PROFILES.len());
    }

    #[test]
    fn by_name_finds_all_profiles() {
        for p in &PROFILES {
            assert!(by_name(p.name).is_some());
        }
        assert!(by_name("nonexistent").is_none());
    }

    #[test]
    fn balanced_engages_all_pairs() {
        let g = by_name("balanced").unwrap().genome();
        assert_eq!(g.pairs.iter().filter(|p| p.engaged()).count(), PAIR_COUNT);
    }

    #[test]
    fn battery_save_engages_power_related_only() {
        let p = by_name("battery-save").unwrap();
        assert_eq!(p.engaged_pairs, &[3, 7, 10, 12, 14]);
        assert_eq!(p.active_count(), 5);
    }

    #[test]
    fn all_profile_indices_in_range() {
        for p in &PROFILES {
            for &k in p.engaged_pairs {
                assert!(k < PAIR_COUNT, "profile {} has invalid pair {}", p.name, k);
            }
        }
    }

    #[test]
    fn genome_is_still_sixty_bytes() {
        for p in &PROFILES {
            assert_eq!(p.genome().to_bytes().len(), 60);
        }
    }

    #[test]
    fn profiles_differ() {
        let a = by_name("balanced").unwrap().genome();
        let b = by_name("battery-save").unwrap().genome();
        assert_ne!(a.to_bytes(), b.to_bytes());
    }
}
