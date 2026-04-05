//! Gate Mesh — per-source 6-axis hexagon projection with 60B genomes.
//!
//! Pure data re-interpretation. No process control, no memory reclamation.

pub mod genome;

pub use genome::{GateGenome, GATE_GENOME_BYTES};
