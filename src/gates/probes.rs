//! Per-gate process aggregators backed by a single `ps` snapshot per tick.

use crate::gates::{GateId, GateGenome, classify};
use crate::gates::genome::{COUNTER_PROCS, COUNTER_RSS_MB, COUNTER_CPU_PCT};
use crate::vitals::Vitals;
use crate::rules::firing;

/// One row from `ps -axm -o rss,pcpu,comm`.
#[derive(Debug, Clone)]
pub struct PsRow {
    pub rss_kb: f64,
    pub cpu_pct: f64,
    pub comm: String,
}

/// Parse the stdout of `ps -axm -o rss=,pcpu=,comm=` into rows.
pub fn parse_ps(stdout: &str) -> Vec<PsRow> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let t = line.trim();
        if t.is_empty() { continue; }
        let mut it = t.split_whitespace();
        let Some(rss_s) = it.next() else { continue };
        let Some(cpu_s) = it.next() else { continue };
        let Ok(rss_kb) = rss_s.parse::<f64>() else { continue };
        let Ok(cpu_pct) = cpu_s.parse::<f64>() else { continue };
        let comm = it.collect::<Vec<_>>().join(" ");
        if comm.is_empty() { continue; }
        out.push(PsRow { rss_kb, cpu_pct, comm });
    }
    out
}

/// Aggregate rss_mb / cpu_pct / procs per gate.
#[derive(Debug, Default, Clone, Copy)]
pub struct GateAgg {
    pub procs: u32,
    pub rss_mb: f64,
    pub cpu_pct: f64,
}

pub fn aggregate(rows: &[PsRow]) -> [GateAgg; 5] {
    let mut aggs = [GateAgg::default(); 5];
    for row in rows {
        let gid = classify(&row.comm);
        let idx = match gid {
            GateId::Macos => 0, GateId::Finder => 1, GateId::Telegram => 2,
            GateId::Chrome => 3, GateId::Safari => 4, GateId::None => continue,
        };
        aggs[idx].procs += 1;
        aggs[idx].rss_mb += row.rss_kb / 1024.0;
        aggs[idx].cpu_pct += row.cpu_pct;
    }
    aggs
}

/// Project one gate aggregate to a `Vitals` sample using simplified axes:
///   cpu   = cpu_pct / 100
///   ram   = rss_mb / total_ram_mb
///   gpu   = 0  (unknown per-gate without private APIs)
///   npu   = 0
///   power = 1  (AC proxy; caller may override)
///   io    = 0
///
/// This matches the projection used by the existing `signature` subcommand
/// so cross-command comparability is preserved.
pub fn project(agg: &GateAgg, total_ram_mb: f64) -> Vitals {
    let ram = if total_ram_mb > 0.0 { (agg.rss_mb / total_ram_mb).clamp(0.0, 1.0) }
              else { 0.0 };
    let cpu = (agg.cpu_pct / 100.0).max(0.0);
    Vitals { ts: 0, axes: [cpu, ram, 0.0, 0.0, 1.0, 0.0] }
}

/// Build a `GateGenome` from an aggregate + timestamp + total RAM.
pub fn genome_for(agg: &GateAgg, total_ram_mb: f64, ts: u32) -> GateGenome {
    let v = project(agg, total_ram_mb);
    let fires = firing(&v);
    let mut g = GateGenome::zeroed();
    for i in 0..6 { g.axes[i] = v.axes[i] as f32; }
    for k in fires { g.set_firing(k, true); }
    g.ts = ts;
    g.counters[COUNTER_PROCS]   = agg.procs as f32;
    g.counters[COUNTER_RSS_MB]  = agg.rss_mb as f32;
    g.counters[COUNTER_CPU_PCT] = agg.cpu_pct as f32;
    g.populate_stats();
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ps_extracts_rows() {
        let stdout = " 1024 5.2 /Applications/Safari.app/Contents/MacOS/Safari\n \
                       512 0.1 /sbin/launchd\n\n \
                       bad line\n";
        let rows = parse_ps(stdout);
        // "bad line" fails because "bad"/"line" are not numeric
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].rss_kb, 1024.0);
        assert_eq!(rows[0].cpu_pct, 5.2);
        assert!(rows[0].comm.contains("Safari"));
    }

    #[test]
    fn aggregate_splits_by_gate() {
        let rows = vec![
            PsRow { rss_kb: 1024.0, cpu_pct: 10.0, comm: "/sbin/launchd".into() },
            PsRow { rss_kb: 2048.0, cpu_pct: 20.0, comm: "/Applications/Google Chrome.app/.../Chrome".into() },
            PsRow { rss_kb:  512.0, cpu_pct:  5.0, comm: "Google Chrome Helper".into() },
            PsRow { rss_kb:  256.0, cpu_pct:  1.0, comm: "/Applications/Telegram.app/.../Telegram".into() },
            PsRow { rss_kb: 9999.0, cpu_pct: 99.0, comm: "some-unrelated-thing".into() },
        ];
        let aggs = aggregate(&rows);
        // index 0 = macos, 3 = chrome, 2 = telegram
        assert_eq!(aggs[0].procs, 1);
        assert!((aggs[0].rss_mb - 1.0).abs() < 1e-3);
        assert_eq!(aggs[3].procs, 2);
        assert!((aggs[3].rss_mb - 2.5).abs() < 1e-3);
        assert!((aggs[3].cpu_pct - 25.0).abs() < 1e-3);
        assert_eq!(aggs[2].procs, 1);
    }

    #[test]
    fn project_scales_axes_correctly() {
        let agg = GateAgg { procs: 3, rss_mb: 8192.0, cpu_pct: 250.0 };
        let v = project(&agg, 16384.0);
        assert!((v.axes[0] - 2.5).abs() < 1e-5);   // cpu = 250/100
        assert!((v.axes[1] - 0.5).abs() < 1e-5);   // ram = 8192/16384
        assert_eq!(v.axes[2], 0.0);                 // gpu unknown
        assert_eq!(v.axes[4], 1.0);                 // power AC proxy
    }

    #[test]
    fn genome_for_preserves_counters_and_fires_rules() {
        // heavy synthetic load: cpu=5.0 (>=4.0), ram=0.95 (>=0.90) → cpu×ram rule fires
        let agg = GateAgg { procs: 3, rss_mb: 15616.0, cpu_pct: 500.0 };
        let g = genome_for(&agg, 16384.0, 1000);
        assert_eq!(g.ts, 1000);
        assert_eq!(g.counters[COUNTER_PROCS], 3.0);
        assert!((g.counters[COUNTER_RSS_MB] - 15616.0).abs() < 1e-3);
        // at least one pair should fire under this load (cpu×ram at minimum)
        assert!(g.firing_count() >= 1, "expected >=1 pair firing, got {}", g.firing_count());
    }
}
