//! Vitals — macOS sensor layer.
//!
//! Read-only wrappers around `sysctl`, `vm_stat`, and `top` to sample the
//! six hexagon axes. All probes are safe: pure `Command::output()` reads,
//! no mutation, no elevated privileges.

use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::SystemTime;

use crate::gate::{Axis, AXIS_COUNT};

/// Single vitals sample — one reading per axis plus a timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vitals {
    /// Unix epoch seconds.
    pub ts: u64,
    /// Per-axis scalar reading, indexed by `Axis::index()`.
    pub axes: [f64; AXIS_COUNT],
}

impl Vitals {
    pub const fn zeroed() -> Self {
        Self { ts: 0, axes: [0.0; AXIS_COUNT] }
    }

    pub fn get(&self, axis: Axis) -> f64 {
        self.axes[axis.index()]
    }
}

/// Run a shell command and return trimmed stdout.
fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// `sysctl -n <key>` returning the raw string value.
pub fn sysctl(key: &str) -> Option<String> {
    run("sysctl", &["-n", key])
}

/// CPU load average (1-minute), as f64. 0.0 on failure.
pub fn cpu_load() -> f64 {
    sysctl("vm.loadavg")
        .and_then(|s| {
            // format: "{ 1.23 2.34 3.45 }"
            s.split_whitespace().nth(1).and_then(|v| v.parse().ok())
        })
        .unwrap_or(0.0)
}

/// Memory pressure fraction in `[0.0, 1.0]` from `memory_pressure` or
/// `vm_stat` fallback.
///
/// Semantic: **0.0 = no pressure (plenty free), 1.0 = system thrashing.**
/// `memory_pressure -Q` reports *free* percentage, so we invert:
/// `pressure = 1.0 - (free% / 100)`.
pub fn ram_pressure() -> f64 {
    // Prefer memory_pressure if available. It prints "free percentage";
    // we invert to get pressure.
    if let Some(out) = run("memory_pressure", &["-Q"]) {
        for line in out.lines() {
            if line.contains("free percentage") {
                if let Some(pct) = line
                    .split(':')
                    .nth(1)
                    .and_then(|v| v.trim().trim_end_matches('%').parse::<f64>().ok())
                {
                    let free = (pct / 100.0).clamp(0.0, 1.0);
                    return (1.0 - free).clamp(0.0, 1.0);
                }
            }
        }
    }
    // Fallback: vm_stat — free / total pages.
    if let Some(vm) = run("vm_stat", &[]) {
        let mut free = 0.0;
        let mut active = 0.0;
        let mut wired = 0.0;
        let mut compressed = 0.0;
        for line in vm.lines() {
            let parse = |l: &str| -> Option<f64> {
                l.split(':').nth(1)?.trim().trim_end_matches('.').parse().ok()
            };
            if line.starts_with("Pages free") {
                free = parse(line).unwrap_or(0.0);
            } else if line.starts_with("Pages active") {
                active = parse(line).unwrap_or(0.0);
            } else if line.starts_with("Pages wired down") {
                wired = parse(line).unwrap_or(0.0);
            } else if line.starts_with("Pages occupied by compressor") {
                compressed = parse(line).unwrap_or(0.0);
            }
        }
        let used = active + wired + compressed;
        let total = used + free;
        if total > 0.0 {
            return (used / total).clamp(0.0, 1.0);
        }
    }
    0.0
}

/// Physical core count (CPU hardware hint — stable, not load).
pub fn cpu_cores() -> f64 {
    sysctl("hw.physicalcpu")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}

/// Number of logical CPUs — proxy for GPU/NPU capability is not directly
/// observable without private APIs, so we report physical core count as a
/// hardware hint for GPU/NPU axes and let the learning loop weight them.
pub fn hw_hint() -> f64 {
    sysctl("hw.ncpu")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}

/// Battery / power: AC power presence as 0.0 or 1.0 (plugged = 1).
pub fn power_ac() -> f64 {
    if let Some(out) = run("pmset", &["-g", "ps"]) {
        if out.contains("AC Power") {
            return 1.0;
        }
    }
    0.0
}

/// IO proxy: page-in rate from vm_stat (pages/sec not available without
/// delta; we return raw swapin count normalized by 1e6).
pub fn io_proxy() -> f64 {
    if let Some(vm) = run("vm_stat", &[]) {
        for line in vm.lines() {
            if line.starts_with("Pageins") {
                if let Some(n) = line
                    .split(':').nth(1)
                    .and_then(|v| v.trim().trim_end_matches('.').parse::<f64>().ok())
                {
                    return (n / 1.0e6).min(1e3);
                }
            }
        }
    }
    0.0
}

/// Collect one vitals sample across all 6 axes.
///
/// GPU and NPU are currently hardware-hint proxies (no stable public API
/// for per-task GPU/NPU utilization on macOS without private frameworks).
pub fn sample() -> Vitals {
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut axes = [0.0; AXIS_COUNT];
    axes[Axis::Cpu.index()] = cpu_load();
    axes[Axis::Ram.index()] = ram_pressure();
    axes[Axis::Gpu.index()] = hw_hint();
    axes[Axis::Npu.index()] = cpu_cores();
    axes[Axis::Power.index()] = power_ac();
    axes[Axis::Io.index()] = io_proxy();

    Vitals { ts, axes }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vitals_zeroed_has_six_axes() {
        let v = Vitals::zeroed();
        assert_eq!(v.axes.len(), 6);
        assert_eq!(v.ts, 0);
        for &a in &v.axes { assert_eq!(a, 0.0); }
    }

    #[test]
    fn get_returns_correct_axis() {
        let mut v = Vitals::zeroed();
        v.axes[Axis::Cpu.index()] = 1.5;
        v.axes[Axis::Ram.index()] = 0.42;
        assert_eq!(v.get(Axis::Cpu), 1.5);
        assert_eq!(v.get(Axis::Ram), 0.42);
    }

    #[test]
    fn sample_returns_6_axes_and_timestamp() {
        let v = sample();
        assert_eq!(v.axes.len(), 6);
        // timestamp is non-zero on any real system.
        assert!(v.ts > 0, "expected non-zero timestamp, got {}", v.ts);
    }

    #[test]
    fn ram_pressure_in_unit_interval() {
        let p = ram_pressure();
        assert!((0.0..=1.0).contains(&p), "ram_pressure out of range: {p}");
    }

    #[test]
    fn power_ac_is_binary() {
        let p = power_ac();
        assert!(p == 0.0 || p == 1.0, "power_ac not binary: {p}");
    }
}
