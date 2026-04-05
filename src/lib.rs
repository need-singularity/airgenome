//! # airgenome
//!
//! A 6-axis resource hexagon for MacBook optimization.
//!
//! ## The core insight
//!
//! Mac resource optimization has a hidden closed-form structure:
//!
//! - **6 axes**: `CPU · RAM · GPU · NPU · POWER · IO`
//! - **15 pair gates**: every unordered axis pair (= `C(6,2)`)
//! - **60-byte genome**: 15 pairs × 4 bytes of learned state
//! - **Banach 1/3 fixed point**: the contraction `I → 0.7·I + 0.1`
//!   converges uniquely to `1/3`; the complement `2/3` is the maximum
//!   achievable "work fraction" of the system.
//!
//! These four numbers `(6, 15, 60, 1/3)` together define a singularity:
//! when all 15 pair gates engage, the efficiency score settles at `2/3`,
//! and the interaction graph's average degree equals `6`.
//!
//! ## Layers
//!
//! - [`gate`] — hexagon topology + 15 pair gates + genome + singularity predicate
//! - [`vitals`] — macOS sensor layer (sysctl / vm_stat / pmset, read-only)
//! - [`efficiency`] — Banach 1/3 fixed-point tracker + mutual-information estimator
//! - [`actuator`] — dry-run actuator with rollback snapshots
//!
//! ## Safety
//!
//! All actuator calls are dry-run by default — proposed changes are
//! recorded, never written to the system. Live actuation is an explicit,
//! opt-in extension.

pub mod gate;
pub mod vitals;
pub mod efficiency;
pub mod actuator;
pub mod rules;
pub mod profile;
pub mod trace;
pub mod buffer;
pub mod policy;

pub use gate::{Axis, Genome, PairGate, ResourceGate, AXIS_COUNT, PAIR_COUNT, GENOME_BYTES, PAIRS};
pub use vitals::{sample, Vitals};
pub use efficiency::{EfficiencyTracker, META_FP, WORK_FP, mutual_info_hist};
pub use actuator::{Action, Actuator, Snapshot};
pub use rules::{Rule, RULES, fires, firing, neighbors, severity, Severity};
pub use profile::{Profile, PROFILES, by_name, names};
pub use trace::{TraceRecord, TraceStats, parse_line, parse_log, summarize};
pub use buffer::{VitalsBuffer, GateMask};
pub use policy::{PolicyEngine, PolicyConfig, Proposal, Reason};
