//! Gate Mesh — per-source 6-axis hexagon projection with 60B genomes.
//!
//! Pure data re-interpretation. No process control, no memory reclamation.

pub mod genome;
pub mod probes;
pub mod log;
pub mod nexus_merger;

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

use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Collect one sample: one `ps` call + `sysctl hw.memsize` for total RAM →
/// produce 5 `GateGenome` records (one per gate).
///
/// Returns `None` if `ps` fails. Zero-allocation paths are not required —
/// this runs once every ~2s.
pub fn sample_all() -> Option<[GateGenome; 5]> {
    let ps_out = Command::new("ps").args(["-axm", "-o", "rss=,pcpu=,comm="])
        .output().ok()?;
    if !ps_out.status.success() { return None; }
    let stdout = String::from_utf8_lossy(&ps_out.stdout);

    let total_ram_mb: f64 = Command::new("sysctl").args(["-n", "hw.memsize"])
        .output().ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<f64>().ok())
        .map(|b| b / 1024.0 / 1024.0)
        .unwrap_or(8192.0);

    let ts = SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as u32).unwrap_or(0);

    let rows = probes::parse_ps(&stdout);
    let aggs = probes::aggregate(&rows);

    let mut out = [GateGenome::zeroed(); 5];
    for i in 0..5 {
        out[i] = probes::genome_for(&aggs[i], total_ram_mb, ts);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_basic() {
        assert_eq!(classify("Telegram.app/Contents/MacOS/Telegram"), GateId::Telegram);
        assert_eq!(classify("launchd"), GateId::Macos);
        assert_eq!(classify("some-random-thing"), GateId::None);
    }

    #[test]
    fn sample_all_returns_5_gates() {
        let r = sample_all().expect("ps should succeed on this host");
        assert_eq!(r.len(), 5);
        // timestamp populated
        assert!(r[0].ts > 0);
        // stats populated
        assert!(r[0].stats[1] >= r[0].stats[0]); // max >= min
    }
}
