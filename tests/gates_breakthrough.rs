//! Integration test: replay the real signatures.jsonl through nexus_merger
//! and verify the breakthrough reproduces.
//!
//! Enhanced with:
//!   - margin threshold (not just adjusted > singularity, but margin >= MARGIN_MIN)
//!   - per-gate + cross-gate decomposition asserts (both must contribute)
//!   - multi-window reproducibility (first half, second half, full history all cross)

use airgenome::gates::nexus_merger::{GateSample, project_breakthrough};

/// Cumulative L4 threshold. L1+L2+L3=+0.142, L4 adds small ~+0.004.
const MARGIN_MIN: f64 = 0.14;

fn parse_signatures(path: &std::path::Path) -> Option<(Vec<GateSample>, Vec<GateSample>, Vec<GateSample>, Vec<GateSample>)> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut macos: Vec<GateSample> = Vec::new();
    let mut finder: Vec<GateSample> = Vec::new();
    let mut telegram: Vec<GateSample> = Vec::new();
    let mut browser: Vec<GateSample> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() { continue; }
        let get = |k: &str| -> Option<&str> {
            let needle = format!("\"{}\":", k);
            let idx = line.find(&needle)?;
            let rest = &line[idx + needle.len()..];
            Some(rest.trim_start().trim_start_matches('"'))
        };
        let ts: u64 = get("ts").and_then(|s|
            s.split(|c: char| !c.is_ascii_digit()).next().unwrap_or("").parse().ok()
        ).unwrap_or(0);
        let cat = get("category").unwrap_or("").split('"').next().unwrap_or("");
        let rss_pct: f64 = get("rss_pct").and_then(|s|
            s.split(|c: char| !(c.is_ascii_digit() || c == '.')).next().unwrap_or("").parse().ok()
        ).unwrap_or(0.0);
        let cpu_pct: f64 = get("cpu_pct").and_then(|s|
            s.split(|c: char| !(c.is_ascii_digit() || c == '.')).next().unwrap_or("").parse().ok()
        ).unwrap_or(0.0);
        let sample = GateSample { ts, ram: rss_pct, cpu: cpu_pct / 100.0 };
        match cat {
            "system" => macos.push(sample),
            "finder" => finder.push(sample),
            "im" => telegram.push(sample),
            "browser" => browser.push(sample),
            _ => {}
        }
    }
    Some((macos, finder, telegram, browser))
}

fn streams_from(ms: Vec<GateSample>, fi: Vec<GateSample>, tg: Vec<GateSample>, br: Vec<GateSample>)
    -> Vec<(String, Vec<GateSample>)> {
    vec![
        ("macos".to_string(), ms),
        ("finder".to_string(), fi),
        ("telegram".to_string(), tg),
        ("browser".to_string(), br),
    ]
}

#[test]
fn signatures_jsonl_crosses_singularity_with_margin() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let path = std::path::PathBuf::from(home).join(".airgenome/signatures.jsonl");
    let Some((macos, finder, telegram, browser)) = parse_signatures(&path) else {
        eprintln!("signatures.jsonl not found — skipping");
        return;
    };
    if macos.len() < 10 || finder.len() < 10 || telegram.len() < 10 || browser.len() < 10 {
        eprintln!("insufficient samples — skipping");
        return;
    }

    let r = project_breakthrough(&streams_from(macos, finder, telegram, browser));
    println!("adjusted={:.4}  singularity={:.4}  distance={:+.4}  margin={:+.4}",
        r.adjusted, r.singularity, r.distance, -r.distance);

    // Gate 1: singularity crossing
    assert!(r.crossed, "singularity not crossed: adjusted={:.4}", r.adjusted);

    // Gate 2: margin threshold
    let margin = r.adjusted - r.singularity;
    assert!(margin >= MARGIN_MIN,
        "margin {:.4} < required {:.4}", margin, MARGIN_MIN);

    // Gate 3: per-gate decomposition must contribute
    assert!(r.per_gate_mi_sum > 0.0,
        "per-gate MI sum should be positive, got {:.4}", r.per_gate_mi_sum);

    // Gate 4: cross-gate coupling must be the dominant NEW signal
    assert!(r.cross_gate_mi_sum > r.per_gate_mi_sum,
        "cross-gate MI ({:.4}) should exceed per-gate MI ({:.4}) — that's the breakthrough engine",
        r.cross_gate_mi_sum, r.per_gate_mi_sum);

    // Gate 5: at least 3 pairwise MI measurements (4 gates → 6 possible pairs, at least 3 valid)
    assert!(r.pair_mi.len() >= 3,
        "expected >=3 valid cross-gate pairs, got {}", r.pair_mi.len());
}

#[test]
fn breakthrough_robust_across_time_windows() {
    // Split history into first/second halves; both should cross.
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let path = std::path::PathBuf::from(home).join(".airgenome/signatures.jsonl");
    let Some((macos, finder, telegram, browser)) = parse_signatures(&path) else {
        eprintln!("signatures.jsonl not found — skipping");
        return;
    };
    if macos.len() < 20 || finder.len() < 20 || telegram.len() < 20 || browser.len() < 20 {
        eprintln!("insufficient samples for window split — skipping");
        return;
    }

    let half = |v: &[GateSample], first: bool| -> Vec<GateSample> {
        let mid = v.len() / 2;
        if first { v[..mid].to_vec() } else { v[mid..].to_vec() }
    };

    for (label, take_first) in [("first-half", true), ("second-half", false)] {
        let streams = streams_from(
            half(&macos, take_first),
            half(&finder, take_first),
            half(&telegram, take_first),
            half(&browser, take_first),
        );
        let r = project_breakthrough(&streams);
        println!("{}: adjusted={:.4} margin={:+.4}", label, r.adjusted, r.adjusted - r.singularity);
        assert!(r.crossed,
            "{} window failed singularity: adjusted={:.4}", label, r.adjusted);
    }
}
