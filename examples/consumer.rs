//! Example: another crate consuming airgenome as a library.
//!
//! Run with:
//! ```sh
//! cargo run --example consumer
//! ```
//!
//! Or as a path dependency from a neighbouring project:
//! ```toml
//! [dependencies]
//! airgenome = { path = "../airgenome" }
//! ```

use airgenome::{
    sample, Vitals, Axis, PolicyEngine, PolicyConfig, Reason,
    by_name, commands_for, firing, severity, Severity,
    PAIR_COUNT, GENOME_BYTES, WORK_FP, META_FP,
};

fn main() {
    println!("airgenome library demo");
    println!("----------------------");
    println!("constants: PAIR_COUNT={}  GENOME_BYTES={}  META_FP={:.6}  WORK_FP={:.6}",
        PAIR_COUNT, GENOME_BYTES, META_FP, WORK_FP);
    println!();

    // 1. Take a single vitals sample.
    let v: Vitals = sample();
    println!("sample @ ts={}:", v.ts);
    for axis in Axis::ALL {
        println!("  {:<6} = {:.4}", axis.name(), v.get(axis));
    }
    println!();

    // 2. Evaluate the 15 rules against those vitals.
    let f = firing(&v);
    println!("firing: {} / {} rules", f.len(), PAIR_COUNT);
    for &k in &f {
        let sev = severity(k, &v);
        let tag = match sev {
            Severity::Critical => "CRIT",
            Severity::Warn => "warn",
            Severity::Ok => "ok",
        };
        println!("  [{:>2}] {}  ({})", k, tag, severity_label(sev));
    }
    println!();

    // 3. Fetch the concrete shell commands for firing pairs.
    println!("suggested actions (user must audit + execute):");
    for &k in f.iter().take(3) {
        if let Some(actions) = commands_for(k) {
            println!("  pair {}: {} remedies", k, actions.len());
            for cmd in actions {
                let prefix = if cmd.needs_sudo { "sudo" } else { "user" };
                println!("    [{}] {}", prefix, cmd.cmd);
            }
        }
    }
    println!();

    // 4. Run a PolicyEngine for a short tick window.
    let cfg = PolicyConfig::default();
    let mut engine = PolicyEngine::new(8, cfg);
    // Feed the same sample three times to satisfy min_samples.
    for _ in 0..3 {
        engine.tick(v);
    }
    let proposals = engine.tick(v);
    println!("policy engine after 4 ticks: {} proposals", proposals.len());
    for p in &proposals {
        let tag = match p.reason { Reason::Reactive => "REACT", Reason::Preemptive => "PREEMP" };
        println!("  {} pair {}: {}", tag, p.pair, p.action);
    }
    println!();

    // 5. Look up a built-in profile and serialize its genome.
    if let Some(battery) = by_name("battery-save") {
        let genome_bytes = battery.genome().to_bytes();
        println!("profile 'battery-save': {} engaged pairs, {}-byte genome",
            battery.active_count(), genome_bytes.len());
    }
}

fn severity_label(s: Severity) -> &'static str {
    match s {
        Severity::Ok => "ok",
        Severity::Warn => "warn",
        Severity::Critical => "critical",
    }
}
