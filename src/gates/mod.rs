//! Gate Mesh — per-source 6-axis hexagon projection with 60B genomes.
//!
//! Pure data re-interpretation. No process control, no memory reclamation.

pub mod genome;
pub mod probes;

pub use genome::{GateGenome, GATE_GENOME_BYTES};

/// Classifier result — which of the 5 gates (if any) this process belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateId { Macos, Finder, Telegram, Chrome, Safari, None }

impl GateId {
    pub const ALL: [GateId; 5] = [
        GateId::Macos, GateId::Finder, GateId::Telegram, GateId::Chrome, GateId::Safari
    ];
    pub fn name(self) -> &'static str {
        match self {
            GateId::Macos => "macos",
            GateId::Finder => "finder",
            GateId::Telegram => "telegram",
            GateId::Chrome => "chrome",
            GateId::Safari => "safari",
            GateId::None => "none",
        }
    }
    pub fn from_name(s: &str) -> Option<GateId> {
        match s {
            "macos" => Some(GateId::Macos),
            "finder" => Some(GateId::Finder),
            "telegram" => Some(GateId::Telegram),
            "chrome" => Some(GateId::Chrome),
            "safari" => Some(GateId::Safari),
            _ => None,
        }
    }
}

/// Classify a process comm/path string into one of the 5 gates.
///
/// Order matters: more specific bundles are checked before `macos` catches
/// all remaining system-adjacent processes. A process matching none of the
/// five returns `GateId::None` and is excluded from the mesh.
pub fn classify(comm: &str) -> GateId {
    let l = comm.to_lowercase();
    // Specific apps first (may contain "apple" or share prefixes)
    if l.contains("telegram") { return GateId::Telegram; }
    // Chrome before Safari before finder (WebKit is used by many apple apps
    // so we require the bundle path to explicitly contain "safari").
    if l.contains("google chrome") || l.contains("chrome helper")
       || l.contains("/chromium") { return GateId::Chrome; }
    if l.contains("/safari.app") || l.contains("safari.app/")
       || l.contains("com.apple.safari.") || l.ends_with("com.apple.safari") {
        return GateId::Safari;
    }
    if l.contains("/finder") || l.contains("com.apple.finder")
       || l.contains("finder.app") { return GateId::Finder; }
    // macOS system processes — core daemons, window server, launchd, etc.
    if l.contains("launchd") || l.contains("windowserver") || l.contains("kernel_task")
       || l.contains("coreservicesd") || l.contains("mdworker") || l.contains("mds_stores")
       || l.contains("loginwindow") || l.contains("systemstats")
       || l.contains("com.apple.") {
        return GateId::Macos;
    }
    GateId::None
}
