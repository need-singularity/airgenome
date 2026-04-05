//! airgenome CLI — probe, status, diagnostics, profile management.

use airgenome::{self, Axis, Genome, AXIS_COUNT, PAIR_COUNT, PAIRS, GENOME_BYTES, RULES};
use std::io::{IsTerminal, Write};

fn use_color() -> bool {
    if std::env::var_os("NO_COLOR").is_some() { return false; }
    std::io::stdout().is_terminal()
}

fn green(s: &str) -> String { if use_color() { format!("\x1b[32m{}\x1b[0m", s) } else { s.to_string() } }
fn yellow(s: &str) -> String { if use_color() { format!("\x1b[33m{}\x1b[0m", s) } else { s.to_string() } }
fn red(s: &str) -> String { if use_color() { format!("\x1b[31m{}\x1b[0m", s) } else { s.to_string() } }
fn dim(s: &str) -> String { if use_color() { format!("\x1b[2m{}\x1b[0m", s) } else { s.to_string() } }

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let sub = args.get(1).map(|s| s.as_str()).unwrap_or("status");

    match sub {
        "status" | "st" => status(&args),
        "probe" | "pr" => probe(),
        "sample" => sample_cmd(&args),
        "pairs" => list_pairs(),
        "rules" => list_rules(),
        "diag" => diag(&args),
        "version" | "--version" | "-V" => {
            println!("airgenome {}", env!("CARGO_PKG_VERSION"));
        }
        "dash" | "dashboard" => dash_cmd(&args),
        "metrics" | "m" => metrics(),
        "explain" => explain_cmd(&args),
        "action" | "actions" => action_cmd(&args),
        "plan" => plan_cmd(&args),
        "apply" => apply_cmd(&args),
        "apply-all" => apply_all_cmd(&args),
        "profile" => profile_cmd(&args),
        "genome" => genome_cmd(&args),
        "diff" => diff_cmd(&args),
        "daemon" => daemon_cmd(&args),
        "trace" => trace_cmd(&args),
        "replay" => replay_cmd(&args),
        "export" => export_cmd(&args),
        "policy" => policy_cmd(&args),
        "init" => init_cmd(&args),
        "uninit" => uninit_cmd(),
        "doctor" | "doc" => doctor_cmd(),
        "summary" | "sum" => summary_cmd(),
        "help" | "-h" | "--help" => print_help(),
        other => {
            eprintln!("unknown sub-command: '{}'", other);
            print_help();
            std::process::exit(2);
        }
    }
}

fn status(args: &[String]) {
    let json = args.iter().any(|a| a == "--json" || a == "-j");
    let v = airgenome::sample();
    let f = airgenome::firing(&v);

    if json {
        print!("{{\"ts\":{},\"axes\":{{", v.ts);
        let mut first = true;
        for axis in Axis::ALL {
            if !first { print!(","); }
            print!("\"{}\":{}", axis.name(), v.get(axis));
            first = false;
        }
        println!("}},\"firing\":{},\"pair_count\":{},\"genome_bytes\":{}}}",
            f.len(), PAIR_COUNT, GENOME_BYTES);
        return;
    }

    println!("=== airgenome — Mac Air Implant Status ===");
    println!("  Hexagon: {} axes × {} pairs | genome = {} bytes",
        AXIS_COUNT, PAIR_COUNT, GENOME_BYTES);
    println!();
    println!("  Axes (vitals sample @ ts={}):", v.ts);
    for axis in Axis::ALL {
        println!("    {:<6} {:>10.4}", axis.name(), v.get(axis));
    }
    println!();
    println!("  Rules firing: {} / {}", f.len(), PAIR_COUNT);
    println!("  Meta fixed point: 1/3 ≈ {:.6}  (work = 2/3 ≈ {:.6})",
        airgenome::META_FP, airgenome::WORK_FP);
}

fn sample_cmd(args: &[String]) {
    let mut count: usize = 1;
    let mut interval_s: u64 = 1;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--count" | "-n" => {
                i += 1;
                if let Some(v) = args.get(i) { count = v.parse().unwrap_or(1).max(1); }
            }
            "--interval" | "-i" => {
                i += 1;
                if let Some(v) = args.get(i) { interval_s = v.parse().unwrap_or(1).max(0); }
            }
            _ => {}
        }
        i += 1;
    }

    print!("[");
    for n in 0..count {
        if n > 0 { print!(","); }
        let v = airgenome::sample();
        let firing = airgenome::firing(&v).len();
        print!("{{\"ts\":{},\"cpu\":{},\"ram\":{},\"gpu\":{},\"npu\":{},\"power\":{},\"io\":{},\"firing\":{}}}",
            v.ts, v.get(Axis::Cpu), v.get(Axis::Ram),
            v.get(Axis::Gpu), v.get(Axis::Npu),
            v.get(Axis::Power), v.get(Axis::Io), firing);
        let _ = std::io::stdout().flush();
        if n + 1 < count && interval_s > 0 {
            std::thread::sleep(std::time::Duration::from_secs(interval_s));
        }
    }
    println!("]");
}

fn probe() {
    let v = airgenome::sample();
    print!("{{\"ts\":{},\"axes\":{{", v.ts);
    let mut first = true;
    for axis in Axis::ALL {
        if !first { print!(","); }
        print!("\"{}\":{}", axis.name(), v.get(axis));
        first = false;
    }
    println!("}}}}");
}

fn list_pairs() {
    println!("Canonical 15 pair gates (C(6,2)):");
    for (i, (a, b)) in PAIRS.iter().enumerate() {
        println!("  [{:>2}] {:<6} × {:<6}", i, a.name(), b.name());
    }
}

fn list_rules() {
    println!("15 rules (triangular mesh, 3 neighbors each):");
    for r in &RULES {
        let n = airgenome::neighbors(r.pair);
        println!("  [{:>2}] {:<14} → neighbors {:?}", r.pair, r.name, n);
        println!("       {}", r.description);
        println!("       {}", r.action);
    }
}

fn diag(args: &[String]) {
    let json = args.iter().any(|a| a == "--json" || a == "-j");
    let v = airgenome::sample();

    if json {
        // per-pair severity array (0=ok,1=warn,2=critical) + firing indices
        print!("{{\"ts\":{},\"severity\":[", v.ts);
        for k in 0..PAIR_COUNT {
            if k > 0 { print!(","); }
            let s = match airgenome::severity(k, &v) {
                airgenome::Severity::Ok => 0,
                airgenome::Severity::Warn => 1,
                airgenome::Severity::Critical => 2,
            };
            print!("{}", s);
        }
        print!("],\"firing\":[");
        let f = airgenome::firing(&v);
        for (i, k) in f.iter().enumerate() {
            if i > 0 { print!(","); }
            print!("{}", k);
        }
        println!("]}}");
        return;
    }

    println!("=== airgenome — Diagnostic ===");
    println!("  ts={}  axes=[cpu={:.2} ram={:.2} gpu={:.0} npu={:.0} power={:.0} io={:.3}]",
        v.ts,
        v.get(Axis::Cpu), v.get(Axis::Ram),
        v.get(Axis::Gpu), v.get(Axis::Npu),
        v.get(Axis::Power), v.get(Axis::Io));
    println!();

    let firing_idx = airgenome::firing(&v);
    let mut critical = 0usize;
    let mut warn = 0usize;

    println!("  Pair Gate Health:");
    for r in &RULES {
        let sev = airgenome::severity(r.pair, &v);
        let tag = match sev {
            airgenome::Severity::Critical => { critical += 1; red("CRITICAL") }
            airgenome::Severity::Warn => { warn += 1; yellow("warn    ") }
            airgenome::Severity::Ok => dim("ok      "),
        };
        println!("    [{:>2}] {:<14} {}", r.pair, r.name, tag);
    }
    println!();
    println!("  Summary: {} firing ({} critical, {} warn, {} ok)",
        firing_idx.len(), critical, warn, PAIR_COUNT - firing_idx.len());

    if !firing_idx.is_empty() {
        println!();
        println!("  Proposed actions (dry-run):");
        for &k in &firing_idx {
            println!("    [{:>2}] {}", k, RULES[k].action);
        }
    }
}

fn apply_all_cmd(args: &[String]) {
    let yes = args.iter().any(|a| a == "--yes" || a == "-y");
    let measure = args.iter().any(|a| a == "--measure" || a == "-m");
    let wait_s: u64 = args.iter().position(|a| a == "--wait").and_then(|i| {
        args.get(i + 1).and_then(|s| s.parse().ok())
    }).unwrap_or(2);

    let before = airgenome::sample();
    let firing = airgenome::firing(&before);
    if firing.is_empty() {
        println!("no rules firing — nothing to apply.");
        return;
    }

    let mut planned = 0usize;
    let mut skipped_no_path = 0usize;
    let mut executed = 0usize;
    let mut advisory = 0usize;
    let mut failed = 0usize;

    println!("=== airgenome apply-all ({} firing) ===", firing.len());
    println!();
    for &k in &firing {
        let (a, b) = PAIRS[k];
        let Some(action) = airgenome::plan_for_pair(k) else {
            println!("  [{:>2}] {}×{}  {}", k, a.name(), b.name(), dim("no Tier 1 path"));
            skipped_no_path += 1;
            continue;
        };
        planned += 1;
        println!("  [{:>2}] {}×{}  → {}", k, a.name(), b.name(), action.label());
        if !yes { continue; }
        match airgenome::execute(&action) {
            Ok(r) if r.skipped => { advisory += 1; }
            Ok(r) => {
                executed += 1;
                if let Some(code) = r.exit_code {
                    println!("       {} exit={}", green("ran"), code);
                }
            }
            Err(e) => {
                failed += 1;
                println!("       {} {:?}", red("abort"), e);
            }
        }
    }

    println!();
    println!("Summary: {} planned · {} no-path · {} advisory · {} executed · {} failed",
        planned, skipped_no_path, advisory, executed, failed);

    if !yes {
        println!();
        println!("{}", dim("dry-run. pass --yes to execute all."));
        return;
    }

    if measure && executed > 0 {
        std::thread::sleep(std::time::Duration::from_secs(wait_s));
        let after = airgenome::sample();
        let before_firing = firing.len();
        let after_firing = airgenome::firing(&after).len();
        println!();
        println!("{}", dim("--- delta ---"));
        println!("  firing  {} → {}  ({:+})", before_firing, after_firing,
            (after_firing as i64) - (before_firing as i64));
        let dram = after.get(Axis::Ram) - before.get(Axis::Ram);
        let dcpu = after.get(Axis::Cpu) - before.get(Axis::Cpu);
        println!("  ram     {:.3} → {:.3}  ({:+.3})",
            before.get(Axis::Ram), after.get(Axis::Ram), dram);
        println!("  cpu     {:.2} → {:.2}  ({:+.2})",
            before.get(Axis::Cpu), after.get(Axis::Cpu), dcpu);
        let verdict = if after_firing < before_firing { green("improved") }
                      else if after_firing > before_firing { red("worse") }
                      else { dim("unchanged") };
        println!("  verdict: {}", verdict);
    }
}

fn apply_cmd(args: &[String]) {
    let pair = args.get(2).and_then(|s| s.parse::<usize>().ok());
    let yes = args.iter().any(|a| a == "--yes" || a == "-y");
    let confirm = args.iter().any(|a| a == "--confirm" || a == "-c");
    let measure = args.iter().any(|a| a == "--measure" || a == "-m");
    let wait_s: u64 = args.iter().position(|a| a == "--wait").and_then(|i| {
        args.get(i + 1).and_then(|s| s.parse().ok())
    }).unwrap_or(2);
    let Some(k) = pair else {
        eprintln!("usage: airgenome apply <pair 0..14> [--yes]");
        std::process::exit(2);
    };
    if k >= PAIR_COUNT {
        eprintln!("pair must be in 0..{}", PAIR_COUNT);
        std::process::exit(2);
    }

    let Some(action) = airgenome::plan_for_pair(k) else {
        eprintln!("no Tier 1 path for pair {}", k);
        std::process::exit(1);
    };
    let Ok(snap) = airgenome::plan(&action) else {
        eprintln!("action refused by safety validator");
        std::process::exit(1);
    };

    let (a, b) = PAIRS[k];
    println!("pair [{}] {}×{}", k, a.name(), b.name());
    println!("action: {}", action.label());
    println!("  {}", snap.observed);

    if !yes && !confirm {
        println!();
        println!("{}", dim("dry-run. pass --yes (or --confirm for interactive prompt)."));
        return;
    }

    // Tier 3: interactive confirmation.
    if confirm && !yes {
        use std::io::{self, Write, BufRead};
        println!();
        print!("execute this action? [y/N] ");
        let _ = io::stdout().flush();
        let stdin = io::stdin();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() {
            eprintln!("read failed");
            std::process::exit(1);
        }
        let answer = line.trim().to_lowercase();
        if answer != "y" && answer != "yes" {
            println!("{}", dim("aborted."));
            return;
        }
    }

    // Measure: sample vitals before execute.
    let before = if measure { Some(airgenome::sample()) } else { None };

    match airgenome::execute(&action) {
        Ok(r) => {
            if r.skipped {
                println!("{}", yellow("skipped (advisory)"));
                return;
            }
            println!();
            println!("executed at ts={} (exit={:?})", r.executed_ts, r.exit_code);
            if !r.stdout.is_empty() { println!("stdout: {}", r.stdout); }
            if !r.stderr.is_empty() { println!("stderr: {}", r.stderr); }

            // Measure: wait, sample after, report delta.
            if let Some(before) = before {
                std::thread::sleep(std::time::Duration::from_secs(wait_s));
                let after = airgenome::sample();
                let before_firing = airgenome::firing(&before).len();
                let after_firing = airgenome::firing(&after).len();
                println!();
                println!("{}", dim("--- delta ---"));
                println!("  firing  {} → {}  ({:+})", before_firing, after_firing,
                    (after_firing as i64) - (before_firing as i64));
                let dram = after.get(Axis::Ram) - before.get(Axis::Ram);
                let dcpu = after.get(Axis::Cpu) - before.get(Axis::Cpu);
                let dio  = after.get(Axis::Io) - before.get(Axis::Io);
                println!("  ram     {:.3} → {:.3}  ({:+.3})",
                    before.get(Axis::Ram), after.get(Axis::Ram), dram);
                println!("  cpu     {:.2} → {:.2}  ({:+.2})",
                    before.get(Axis::Cpu), after.get(Axis::Cpu), dcpu);
                println!("  io      {:.3} → {:.3}  ({:+.3})",
                    before.get(Axis::Io), after.get(Axis::Io), dio);
                let verdict = if after_firing < before_firing { green("improved") }
                              else if after_firing > before_firing { red("worse") }
                              else { dim("unchanged") };
                println!("  verdict: {}", verdict);
            }
            // Append to apply.log
            let log = home_dir().join(".airgenome").join("apply.log");
            let _ = std::fs::create_dir_all(log.parent().unwrap());
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&log) {
                let _ = writeln!(f,
                    "{{\"ts\":{},\"pair\":{},\"kind\":\"{}\",\"target\":\"{}\",\"exit\":{:?},\"skipped\":{}}}",
                    r.executed_ts, k, r.pre.kind,
                    r.pre.target.replace('"', "\\\""),
                    r.exit_code, r.skipped);
            }
        }
        Err(e) => {
            eprintln!("execute failed: {:?}", e);
            std::process::exit(1);
        }
    }
}

fn plan_cmd(_args: &[String]) {
    let v = airgenome::sample();
    let firing = airgenome::firing(&v);
    if firing.is_empty() {
        println!("no rules firing — no Tier 1 plan needed.");
        return;
    }

    println!("=== airgenome — Tier 1 plan (dry-run) ===");
    println!();
    for &k in &firing {
        let (a, b) = PAIRS[k];
        match airgenome::plan_for_pair(k) {
            Some(action) => {
                match airgenome::plan(&action) {
                    Ok(snap) => {
                        println!("  [{:>2}] {}×{}", k, a.name(), b.name());
                        println!("       action: {}", action.label());
                        println!("       {}", dim(&format!("cmd: {}", snap.observed)));
                    }
                    Err(e) => {
                        println!("  [{:>2}] {}×{} — {} {:?}", k, a.name(), b.name(), red("abort:"), e);
                    }
                }
            }
            None => {
                println!("  [{:>2}] {}×{}  {}", k, a.name(), b.name(), dim("(no Tier 1 path)"));
            }
        }
    }
    println!();
    println!("{}", dim("Tier 1 plans are dry-run; no action is executed."));
}

fn action_cmd(args: &[String]) {
    let json = args.iter().any(|a| a == "--json" || a == "-j");
    let no_sudo = args.iter().any(|a| a == "--no-sudo");
    let sudo_only = args.iter().any(|a| a == "--sudo-only");
    // if no pair given (and no flags), show actions for all currently-firing pairs
    let target = args.iter().skip(2).find_map(|s| {
        if s.starts_with('-') { None } else { s.parse::<usize>().ok() }
    });
    let v = airgenome::sample();

    let pairs: Vec<usize> = match target {
        Some(k) if k < PAIR_COUNT => vec![k],
        Some(_) => { eprintln!("pair must be in 0..{}", PAIR_COUNT); std::process::exit(2); }
        None => airgenome::firing(&v),
    };

    let keep = |c: &airgenome::ActionCommand| -> bool {
        if no_sudo && c.needs_sudo { return false; }
        if sudo_only && !c.needs_sudo { return false; }
        true
    };

    if json {
        print!("{{\"pairs\":[");
        for (i, &k) in pairs.iter().enumerate() {
            if i > 0 { print!(","); }
            print!("{{\"pair\":{},\"commands\":[", k);
            if let Some(actions) = airgenome::commands_for(k) {
                let mut first = true;
                for c in actions.iter().filter(|c| keep(c)) {
                    if !first { print!(","); }
                    first = false;
                    let esc = c.cmd.replace('\\', "\\\\").replace('"', "\\\"");
                    let eff = c.effect.replace('\\', "\\\\").replace('"', "\\\"");
                    print!("{{\"cmd\":\"{}\",\"effect\":\"{}\",\"sudo\":{}}}",
                        esc, eff, c.needs_sudo);
                }
            }
            print!("]}}");
        }
        println!("]}}");
        return;
    }

    if pairs.is_empty() {
        println!("no pairs firing — nothing to act on.");
        return;
    }

    for k in pairs {
        let (a, b) = PAIRS[k];
        println!("[{:>2}] {}×{}  ({})", k, a.name(), b.name(), RULES[k].description);
        if let Some(actions) = airgenome::commands_for(k) {
            for c in actions.iter().filter(|c| keep(c)) {
                let tag = if c.needs_sudo { red("sudo") } else { dim("user") };
                println!("  [{}] {}", tag, c.cmd);
                println!("       → {}", c.effect);
            }
        }
        println!();
    }
    println!("{}", dim("airgenome never runs these commands itself — audit then execute."));
}

fn explain_cmd(args: &[String]) {
    let Some(arg) = args.get(2) else {
        eprintln!("usage: airgenome explain <pair-index 0..14>");
        std::process::exit(2);
    };
    let k: usize = match arg.parse() {
        Ok(n) if n < PAIR_COUNT => n,
        _ => {
            eprintln!("pair index must be in 0..{}", PAIR_COUNT);
            std::process::exit(2);
        }
    };

    let r = &RULES[k];
    let (ax, bx) = PAIRS[k];
    let n = airgenome::neighbors(k);
    let v = airgenome::sample();
    let a_val = v.get(ax);
    let b_val = v.get(bx);
    let firing = airgenome::fires(k, &v);
    let sev = airgenome::severity(k, &v);
    let sev_str = match sev {
        airgenome::Severity::Ok => "ok",
        airgenome::Severity::Warn => "warn",
        airgenome::Severity::Critical => "CRITICAL",
    };

    println!("=== Pair Gate [{}] — {} × {} ===", k, ax.name(), bx.name());
    println!();
    println!("  Description:");
    println!("    {}", r.description);
    println!();
    println!("  Proposed action (dry-run):");
    println!("    {}", r.action);
    println!();
    println!("  Mesh neighbors: pair {:?}", n);
    for &m in &n {
        let (mx, my) = PAIRS[m];
        println!("    [{:>2}] {}×{}: {}", m, mx.name(), my.name(), RULES[m].description);
    }
    println!();
    println!("  Current vitals:");
    println!("    {:<6} = {:>6.2}", ax.name(), a_val);
    println!("    {:<6} = {:>6.2}", bx.name(), b_val);
    println!();
    println!("  State: {} (firing={})", sev_str, firing);
}

fn metrics() {
    let v = airgenome::sample();
    let firing = airgenome::firing(&v);
    let firing_count = firing.len();
    let work_fraction = 1.0 - (firing_count as f64) / (PAIR_COUNT as f64);

    // Prometheus text format (https://prometheus.io/docs/instrumenting/exposition_formats/)
    println!("# HELP airgenome_axis_value Current value of a hexagon axis.");
    println!("# TYPE airgenome_axis_value gauge");
    for axis in Axis::ALL {
        println!("airgenome_axis_value{{axis=\"{}\"}} {}", axis.name(), v.get(axis));
    }

    println!("# HELP airgenome_firing_total Number of rules currently firing.");
    println!("# TYPE airgenome_firing_total gauge");
    println!("airgenome_firing_total {}", firing_count);

    println!("# HELP airgenome_pair_count Total number of pair gates.");
    println!("# TYPE airgenome_pair_count gauge");
    println!("airgenome_pair_count {}", PAIR_COUNT);

    println!("# HELP airgenome_work_fraction Work fraction (1 - firing/15).");
    println!("# TYPE airgenome_work_fraction gauge");
    println!("airgenome_work_fraction {}", work_fraction);

    println!("# HELP airgenome_work_fraction_ceiling Theoretical 2/3 ceiling.");
    println!("# TYPE airgenome_work_fraction_ceiling gauge");
    println!("airgenome_work_fraction_ceiling {}", airgenome::WORK_FP);

    println!("# HELP airgenome_pair_severity Per-pair severity: 0=ok 1=warn 2=critical.");
    println!("# TYPE airgenome_pair_severity gauge");
    for k in 0..PAIR_COUNT {
        let (a, b) = PAIRS[k];
        let sev = match airgenome::severity(k, &v) {
            airgenome::Severity::Ok => 0,
            airgenome::Severity::Warn => 1,
            airgenome::Severity::Critical => 2,
        };
        println!("airgenome_pair_severity{{pair=\"{}\",a=\"{}\",b=\"{}\"}} {}",
            k, a.name(), b.name(), sev);
    }

    println!("# HELP airgenome_sample_timestamp Unix timestamp of the vitals sample.");
    println!("# TYPE airgenome_sample_timestamp gauge");
    println!("airgenome_sample_timestamp {}", v.ts);
}

fn dash_cmd(args: &[String]) {
    let mut watch = false;
    let mut interval_s: u64 = 2;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--watch" | "-w" => watch = true,
            "--interval" | "-i" => {
                i += 1;
                if let Some(v) = args.get(i) { interval_s = v.parse().unwrap_or(2).max(1); }
            }
            _ => {}
        }
        i += 1;
    }
    if !watch {
        dash();
        return;
    }
    // Watch mode: clear screen and redraw.
    loop {
        // ESC[2J clears screen; ESC[H moves cursor to home.
        print!("\x1b[2J\x1b[H");
        let _ = std::io::stdout().flush();
        dash();
        println!("  (refresh every {}s — Ctrl+C to exit)", interval_s);
        std::thread::sleep(std::time::Duration::from_secs(interval_s));
    }
}

fn dash() {
    let v = airgenome::sample();
    let fire_set: std::collections::HashSet<usize> =
        airgenome::firing(&v).into_iter().collect();

    // helper: 10-cell bar for a value in [0, max].
    fn bar(val: f64, max: f64) -> String {
        let filled = ((val / max).clamp(0.0, 1.0) * 10.0).round() as usize;
        let mut s = String::with_capacity(12);
        s.push('[');
        for i in 0..10 {
            s.push(if i < filled { '█' } else { '░' });
        }
        s.push(']');
        s
    }

    println!("┌─ airgenome — hexagon dashboard ────────────────────────┐");
    println!("│                                                        │");
    println!("│  Axes:                                                 │");
    let cpu = v.get(Axis::Cpu);
    let ram = v.get(Axis::Ram);
    let gpu = v.get(Axis::Gpu);
    let npu = v.get(Axis::Npu);
    let pow = v.get(Axis::Power);
    let io  = v.get(Axis::Io);
    println!("│    cpu   {} {:>6.2}                          │", bar(cpu, 8.0), cpu);
    println!("│    ram   {} {:>6.2}                          │", bar(ram, 1.0), ram);
    println!("│    gpu   {} {:>6.2}                          │", bar(gpu, 8.0), gpu);
    println!("│    npu   {} {:>6.2}                          │", bar(npu, 8.0), npu);
    println!("│    power {} {:>6.2}                          │", bar(pow, 1.0), pow);
    println!("│    io    {} {:>6.2}                          │", bar(io,  3.0), io);
    println!("│                                                        │");
    println!("│  15 Pair Gates:                                        │");

    let cells: Vec<String> = (0..PAIR_COUNT).map(|k| {
        let sev = airgenome::severity(k, &v);
        let tag = match sev {
            airgenome::Severity::Ok => "ok ",
            airgenome::Severity::Warn => "wrn",
            airgenome::Severity::Critical => "CRI",
        };
        let (a, b) = PAIRS[k];
        let short_a: String = a.name().chars().take(3).collect();
        let short_b: String = b.name().chars().take(3).collect();
        format!("[{:>2}{:>3}×{:<3}{}]", k, short_a, short_b, tag)
    }).collect();

    for chunk in cells.chunks(3) {
        print!("│  ");
        for c in chunk { print!(" {}", c); }
        // pad row to card width
        let used = chunk.len();
        for _ in used..3 { print!("            "); }
        println!(" │");
    }

    let firing_count = fire_set.len();
    let work_fraction = 1.0 - (firing_count as f64) / (PAIR_COUNT as f64);

    println!("│                                                        │");
    println!("│  Firing: {:>2}/{}    Work fraction: {:.3}  (ceil {:.3})  │",
        firing_count, PAIR_COUNT, work_fraction, airgenome::WORK_FP);
    println!("│                                                        │");

    // ascii bar: one cell per pair
    print!("│  ");
    for k in 0..PAIR_COUNT {
        let cell = match airgenome::severity(k, &v) {
            airgenome::Severity::Ok => dim("·"),
            airgenome::Severity::Warn => yellow("▒"),
            airgenome::Severity::Critical => red("█"),
        };
        print!("{}", cell);
        print!(" ");
    }
    for _ in 0..(54 - 2*PAIR_COUNT - 2) { print!(" "); }
    println!("│");
    println!("│  ({} ok   {} warn   {} critical)                          │",
        dim("·"), yellow("▒"), red("█"));
    println!("└────────────────────────────────────────────────────────┘");
}

fn genome_cmd(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("help");
    match sub {
        "save" => genome_save(args.get(3).map(|s| s.as_str()), args.get(4).map(|s| s.as_str())),
        "cat" | "show" => genome_cat(args.get(3).map(|s| s.as_str())),
        "hex" => genome_hex(args.get(3).map(|s| s.as_str())),
        "from-hex" => genome_from_hex(args.get(3).map(|s| s.as_str()),
                                      args.get(4).map(|s| s.as_str())),
        _ => {
            eprintln!("usage: airgenome genome <save|cat|hex|from-hex> [args]");
            eprintln!("  save     <profile> <path>    write built-in profile's 60 bytes to file");
            eprintln!("  cat      <path>              display a genome file");
            eprintln!("  hex      <profile>           print 60-byte genome as hex");
            eprintln!("  from-hex <120-char> [path]   parse hex; write to path (or display)");
            std::process::exit(2);
        }
    }
}

fn genome_save(profile: Option<&str>, path: Option<&str>) {
    let (Some(name), Some(path)) = (profile, path) else {
        eprintln!("usage: airgenome genome save <profile> <path>");
        std::process::exit(2);
    };
    let Some(p) = airgenome::by_name(name) else {
        eprintln!("unknown profile: {}", name);
        std::process::exit(2);
    };
    let bytes = p.genome().to_bytes();
    if let Err(e) = std::fs::write(path, bytes) {
        eprintln!("cannot write {}: {}", path, e);
        std::process::exit(1);
    }
    println!("saved profile '{}' ({} pairs) → {}", name, p.active_count(), path);
}

fn genome_cat(path: Option<&str>) {
    let Some(path) = path else {
        eprintln!("usage: airgenome genome cat <path>");
        std::process::exit(2);
    };
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => { eprintln!("cannot read {}: {}", path, e); std::process::exit(1); }
    };
    if bytes.len() != GENOME_BYTES {
        eprintln!("bad genome: expected {} bytes, got {}", GENOME_BYTES, bytes.len());
        std::process::exit(1);
    }
    let mut arr = [0u8; GENOME_BYTES];
    arr.copy_from_slice(&bytes);
    let g = airgenome::Genome::from_bytes(&arr);

    // match against built-ins
    let mut name = "(custom)";
    for p in airgenome::PROFILES.iter() {
        if p.genome() == g { name = p.name; break; }
    }
    let active: Vec<usize> = (0..PAIR_COUNT).filter(|&k| g.pairs[k].engaged()).collect();
    println!("Genome: {}", path);
    println!("  matches: {}", name);
    println!("  engaged pairs ({}): {:?}", active.len(), active);
    print!("  hex: ");
    for b in bytes.iter() { print!("{:02x}", b); }
    println!();
}

fn genome_from_hex(hex: Option<&str>, path: Option<&str>) {
    let Some(hex) = hex else {
        eprintln!("usage: airgenome genome from-hex <120-char> [path]");
        std::process::exit(2);
    };
    let hex = hex.trim();
    if hex.len() != GENOME_BYTES * 2 {
        eprintln!("hex must be exactly {} chars (got {})", GENOME_BYTES * 2, hex.len());
        std::process::exit(2);
    }
    let mut bytes = [0u8; GENOME_BYTES];
    for i in 0..GENOME_BYTES {
        let s = &hex[i*2..i*2+2];
        match u8::from_str_radix(s, 16) {
            Ok(b) => bytes[i] = b,
            Err(_) => {
                eprintln!("invalid hex at position {}: '{}'", i*2, s);
                std::process::exit(2);
            }
        }
    }

    let g = airgenome::Genome::from_bytes(&bytes);
    let mut name = "(custom)";
    for p in airgenome::PROFILES.iter() {
        if p.genome() == g { name = p.name; break; }
    }
    let active: Vec<usize> = (0..PAIR_COUNT).filter(|&k| g.pairs[k].engaged()).collect();

    if let Some(path) = path {
        if let Err(e) = std::fs::write(path, bytes) {
            eprintln!("cannot write {}: {}", path, e);
            std::process::exit(1);
        }
        println!("wrote {} bytes to {}", GENOME_BYTES, path);
        println!("  matches: {}", name);
        println!("  engaged pairs ({}): {:?}", active.len(), active);
    } else {
        println!("Genome (from hex):");
        println!("  matches: {}", name);
        println!("  engaged pairs ({}): {:?}", active.len(), active);
    }
}

fn genome_hex(profile: Option<&str>) {
    let Some(name) = profile else {
        eprintln!("usage: airgenome genome hex <profile>");
        std::process::exit(2);
    };
    let Some(p) = airgenome::by_name(name) else {
        eprintln!("unknown profile: {}", name);
        std::process::exit(2);
    };
    let bytes = p.genome().to_bytes();
    for b in bytes.iter() { print!("{:02x}", b); }
    println!();
}

fn profile_cmd(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("list");
    match sub {
        "list" | "ls" => profile_list(),
        "show" => profile_show(args.get(3).map(|s| s.as_str())),
        "apply" => profile_apply(args.get(3).map(|s| s.as_str())),
        "active" => profile_active(),
        other => {
            eprintln!("unknown profile sub-command: '{}'", other);
            eprintln!("usage: airgenome profile [list|show <name>|apply <name>]");
            std::process::exit(2);
        }
    }
}

fn profile_list() {
    println!("Built-in profiles ({}):", airgenome::PROFILES.len());
    for p in airgenome::PROFILES.iter() {
        println!("  {:<14} {:>2} pairs  — {}",
            p.name, p.active_count(), p.description);
    }

    // Scan user profiles from ~/.airgenome/profiles/*.genome.
    let user_dir = home_dir().join(".airgenome").join("profiles");
    let entries = match std::fs::read_dir(&user_dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    let mut users: Vec<(String, usize)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("genome") { continue; }
        let Ok(bytes) = std::fs::read(&path) else { continue; };
        if bytes.len() != GENOME_BYTES { continue; }
        let mut arr = [0u8; GENOME_BYTES];
        arr.copy_from_slice(&bytes);
        let g = airgenome::Genome::from_bytes(&arr);
        let active = (0..PAIR_COUNT).filter(|&k| g.pairs[k].engaged()).count();
        let stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        users.push((stem, active));
    }
    if !users.is_empty() {
        users.sort();
        println!();
        println!("User profiles ({}):  {}", users.len(), user_dir.display());
        for (name, active) in &users {
            println!("  {:<14} {:>2} pairs", name, active);
        }
    }
}

fn profile_show(name: Option<&str>) {
    let Some(name) = name else {
        eprintln!("usage: airgenome profile show <name>");
        std::process::exit(2);
    };
    let Some(p) = airgenome::by_name(name) else {
        eprintln!("unknown profile: '{}'", name);
        eprintln!("run `airgenome profile list` to see available profiles");
        std::process::exit(2);
    };
    let g = p.genome();
    let bytes = g.to_bytes();
    println!("Profile: {}", p.name);
    println!("  {}", p.description);
    println!("  Engaged pairs ({}):", p.active_count());
    for &k in p.engaged_pairs {
        let (a, b) = PAIRS[k];
        println!("    [{:>2}] {}×{}", k, a.name(), b.name());
    }
    print!("  Genome (60 bytes hex): ");
    for b in bytes.iter() { print!("{:02x}", b); }
    println!();
}

fn profile_apply(name: Option<&str>) {
    let Some(name) = name else {
        eprintln!("usage: airgenome profile apply <name-or-path>");
        std::process::exit(2);
    };

    let dir = home_dir().join(".airgenome");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("cannot create {}: {}", dir.display(), e);
        std::process::exit(1);
    }
    let active = dir.join("active.genome");

    // Resolve `name` as one of:
    //   1. built-in profile name
    //   2. user profile at ~/.airgenome/profiles/<name>.genome
    //   3. filesystem path to a .genome file
    let (bytes, source): (Vec<u8>, String) = if let Some(p) = airgenome::by_name(name) {
        (p.genome().to_bytes().to_vec(),
         format!("built-in profile '{}' ({} pairs)", p.name, p.active_count()))
    } else {
        let user_path = dir.join("profiles").join(format!("{}.genome", name));
        let candidate = if user_path.exists() {
            user_path
        } else {
            std::path::PathBuf::from(name)
        };
        let bytes = match std::fs::read(&candidate) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("'{}' is neither a built-in profile name nor a readable file: {}",
                    name, e);
                eprintln!("available built-ins:");
                for p in airgenome::PROFILES { eprintln!("  {}", p.name); }
                std::process::exit(2);
            }
        };
        if bytes.len() != GENOME_BYTES {
            eprintln!("bad genome at {}: expected {} bytes, got {}",
                candidate.display(), GENOME_BYTES, bytes.len());
            std::process::exit(1);
        }
        let mut arr = [0u8; GENOME_BYTES];
        arr.copy_from_slice(&bytes);
        let g = airgenome::Genome::from_bytes(&arr);
        let active_count = (0..PAIR_COUNT).filter(|&k| g.pairs[k].engaged()).count();
        (bytes, format!("custom genome at {} ({} pairs)", candidate.display(), active_count))
    };

    if let Err(e) = std::fs::write(&active, &bytes) {
        eprintln!("cannot write {}: {}", active.display(), e);
        std::process::exit(1);
    }
    println!("applied: {}", source);
    println!("  → {}", active.display());
    println!("  (dry-run: this records the selection; no sysctl changes are made)");
}

fn profile_active() {
    let path = home_dir().join(".airgenome").join("active.genome");
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("no active profile at {} ({})", path.display(), e);
            eprintln!("run `airgenome profile apply <name>` to set one");
            std::process::exit(1);
        }
    };
    if bytes.len() != GENOME_BYTES {
        eprintln!("corrupt genome: expected {} bytes, got {}", GENOME_BYTES, bytes.len());
        std::process::exit(1);
    }
    let mut arr = [0u8; GENOME_BYTES];
    arr.copy_from_slice(&bytes);
    let g = Genome::from_bytes(&arr);

    // Try to match against built-in profiles.
    let mut name = "(custom)";
    for p in airgenome::PROFILES.iter() {
        if p.genome() == g { name = p.name; break; }
    }
    println!("Active profile: {}", name);
    println!("  Path: {}", path.display());
    let active: Vec<usize> = (0..PAIR_COUNT)
        .filter(|&k| g.pairs[k].engaged())
        .collect();
    println!("  Engaged pairs ({}): {:?}", active.len(), active);
    print!("  Genome hex: ");
    for b in bytes.iter() { print!("{:02x}", b); }
    println!();
}

fn diff_cmd(args: &[String]) {
    let a = args.get(2).map(|s| s.as_str());
    let b = args.get(3).map(|s| s.as_str());
    let (Some(a), Some(b)) = (a, b) else {
        eprintln!("usage: airgenome diff <profile-a> <profile-b>");
        std::process::exit(2);
    };
    let pa = match airgenome::by_name(a) {
        Some(p) => p,
        None => { eprintln!("unknown profile: {}", a); std::process::exit(2); }
    };
    let pb = match airgenome::by_name(b) {
        Some(p) => p,
        None => { eprintln!("unknown profile: {}", b); std::process::exit(2); }
    };

    let ga = pa.genome().to_bytes();
    let gb = pb.genome().to_bytes();
    let mut diff_pairs = 0usize;

    println!("diff: {} → {}", a, b);
    for k in 0..PAIR_COUNT {
        let sa = u32::from_le_bytes([ga[k*4], ga[k*4+1], ga[k*4+2], ga[k*4+3]]);
        let sb = u32::from_le_bytes([gb[k*4], gb[k*4+1], gb[k*4+2], gb[k*4+3]]);
        if sa != sb {
            diff_pairs += 1;
            let (ax, bx) = PAIRS[k];
            println!("  [{:>2}] {}×{}  0x{:08x} → 0x{:08x}",
                k, ax.name(), bx.name(), sa, sb);
        }
    }
    println!("{} / {} pairs differ", diff_pairs, PAIR_COUNT);
}

fn daemon_cmd(args: &[String]) {
    // parse --interval N (seconds), --output PATH, --once
    let mut interval_s: u64 = 30;
    let mut output: Option<std::path::PathBuf> = None;
    let mut once = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--interval" | "-i" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    interval_s = v.parse().unwrap_or(30).max(1);
                }
            }
            "--output" | "-o" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    output = Some(std::path::PathBuf::from(v));
                }
            }
            "--once" => once = true,
            _ => {}
        }
        i += 1;
    }

    let out_path = output.unwrap_or_else(|| {
        let dir = home_dir().join(".airgenome");
        let _ = std::fs::create_dir_all(&dir);
        dir.join("vitals.jsonl")
    });

    let mut file = match std::fs::OpenOptions::new()
        .create(true).append(true).open(&out_path)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("cannot open {}: {}", out_path.display(), e);
            std::process::exit(1);
        }
    };

    if !once {
        eprintln!("airgenome daemon — interval {}s, log {}", interval_s, out_path.display());
        eprintln!("Ctrl+C to stop.");
    }

    let write_one = |file: &mut std::fs::File| -> bool {
        let v = airgenome::sample();
        let firing = airgenome::firing(&v).len();
        let line = format!(
            "{{\"ts\":{},\"cpu\":{},\"ram\":{},\"gpu\":{},\"npu\":{},\"power\":{},\"io\":{},\"firing\":{}}}",
            v.ts, v.get(Axis::Cpu), v.get(Axis::Ram),
            v.get(Axis::Gpu), v.get(Axis::Npu),
            v.get(Axis::Power), v.get(Axis::Io), firing
        );
        if writeln!(file, "{}", line).is_err() {
            eprintln!("write failed");
            return false;
        }
        let _ = file.flush();
        if !once {
            eprintln!("[{}] firing={}/{}  cpu={:.2} ram={:.2} io={:.2}",
                v.ts, firing, PAIR_COUNT,
                v.get(Axis::Cpu), v.get(Axis::Ram), v.get(Axis::Io));
        }
        true
    };

    if once {
        if !write_one(&mut file) { std::process::exit(1); }
        return;
    }

    loop {
        if !write_one(&mut file) { std::process::exit(1); }
        std::thread::sleep(std::time::Duration::from_secs(interval_s));
    }
}

fn trace_cmd(args: &[String]) {
    // parse --input PATH, --tail N, --json
    let mut input: Option<std::path::PathBuf> = None;
    let mut tail: Option<usize> = None;
    let mut json = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--input" | "-i" => {
                i += 1;
                if let Some(v) = args.get(i) { input = Some(std::path::PathBuf::from(v)); }
            }
            "--tail" | "-t" => {
                i += 1;
                if let Some(v) = args.get(i) { tail = v.parse().ok(); }
            }
            "--json" | "-j" => json = true,
            _ => {}
        }
        i += 1;
    }

    let path = input.unwrap_or_else(|| home_dir().join(".airgenome").join("vitals.jsonl"));
    let body = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {}: {}", path.display(), e);
            std::process::exit(1);
        }
    };

    let mut records = airgenome::parse_log(&body);
    if let Some(n) = tail {
        if records.len() > n {
            records = records[records.len() - n..].to_vec();
        }
    }

    let stats = airgenome::summarize(&records);

    if json {
        let invalid = body.lines().count().saturating_sub(stats.count);
        println!("{{\"source\":\"{}\",\"records\":{},\"invalid\":{},\"span_secs\":{},\"cpu_mean\":{},\"ram_mean\":{},\"io_mean\":{},\"firing_mean\":{},\"firing_max\":{},\"work_fraction\":{},\"on_battery_frac\":{}}}",
            path.display().to_string().replace('\\', "\\\\").replace('"', "\\\""),
            stats.count, invalid, stats.span_secs,
            stats.cpu_mean, stats.ram_mean, stats.io_mean,
            stats.firing_mean, stats.firing_max, stats.work_fraction, stats.on_battery_frac);
        return;
    }

    println!("=== airgenome — Trace Summary ===");
    println!("  Source : {}", path.display());
    println!("  Records: {}  ({} invalid lines skipped)",
        stats.count,
        body.lines().count().saturating_sub(stats.count));
    if stats.count == 0 {
        println!("  (no valid records — run `airgenome daemon` for a while first)");
        return;
    }
    let hours = stats.span_secs as f64 / 3600.0;
    println!("  Span   : {}s ({:.2} h)", stats.span_secs, hours);
    println!();
    println!("  Means:");
    println!("    cpu load      {:>6.2}", stats.cpu_mean);
    println!("    ram pressure  {:>6.2}", stats.ram_mean);
    println!("    io proxy      {:>6.2}", stats.io_mean);
    println!();
    println!("  Firing:");
    println!("    mean          {:>5.2} / {}", stats.firing_mean, airgenome::PAIR_COUNT);
    println!("    max           {:>5}", stats.firing_max);
    println!("    work fraction {:>5.3}  (ceiling 2/3 ≈ {:.3})",
        stats.work_fraction, airgenome::WORK_FP);
    println!();
    println!("  Battery: {:.1}% of samples on battery", stats.on_battery_frac * 100.0);
}

fn export_cmd(args: &[String]) {
    let mut input: Option<std::path::PathBuf> = None;
    let mut output: Option<std::path::PathBuf> = None;
    let mut format = "csv".to_string();
    let mut tail: Option<usize> = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--input" | "-i" => { i += 1; if let Some(v) = args.get(i) { input = Some(std::path::PathBuf::from(v)); } }
            "--output" | "-o" => { i += 1; if let Some(v) = args.get(i) { output = Some(std::path::PathBuf::from(v)); } }
            "--format" | "-f" => { i += 1; if let Some(v) = args.get(i) { format = v.clone(); } }
            "--tail" | "-t" => { i += 1; if let Some(v) = args.get(i) { tail = v.parse().ok(); } }
            _ => {}
        }
        i += 1;
    }

    let path = input.unwrap_or_else(|| home_dir().join(".airgenome").join("vitals.jsonl"));
    let body = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => { eprintln!("cannot read {}: {}", path.display(), e); std::process::exit(1); }
    };
    let mut records = airgenome::parse_log(&body);
    if let Some(n) = tail {
        if records.len() > n { records = records[records.len() - n..].to_vec(); }
    }

    let mut out: String = String::new();
    match format.as_str() {
        "csv" => {
            out.push_str("ts,cpu,ram,gpu,npu,power,io,firing\n");
            for r in &records {
                out.push_str(&format!("{},{},{},{},{},{},{},{}\n",
                    r.ts, r.cpu, r.ram, r.gpu, r.npu, r.power, r.io, r.firing));
            }
        }
        "json" => {
            out.push('[');
            for (i, r) in records.iter().enumerate() {
                if i > 0 { out.push(','); }
                out.push_str(&format!(
                    "{{\"ts\":{},\"cpu\":{},\"ram\":{},\"gpu\":{},\"npu\":{},\"power\":{},\"io\":{},\"firing\":{}}}",
                    r.ts, r.cpu, r.ram, r.gpu, r.npu, r.power, r.io, r.firing));
            }
            out.push_str("]\n");
        }
        other => {
            eprintln!("unknown format '{}': use csv or json", other);
            std::process::exit(2);
        }
    }

    match output {
        Some(p) => {
            if let Err(e) = std::fs::write(&p, &out) {
                eprintln!("cannot write {}: {}", p.display(), e);
                std::process::exit(1);
            }
            eprintln!("wrote {} records ({}) → {}", records.len(), format, p.display());
        }
        None => { print!("{}", out); }
    }
}

fn replay_cmd(args: &[String]) {
    let mut input: Option<std::path::PathBuf> = None;
    let mut capacity: usize = 12;
    let mut verbose = false;
    let mut json = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--input" | "-i" => {
                i += 1;
                if let Some(v) = args.get(i) { input = Some(std::path::PathBuf::from(v)); }
            }
            "--capacity" | "-c" => {
                i += 1;
                if let Some(v) = args.get(i) { capacity = v.parse().unwrap_or(12).max(3); }
            }
            "--verbose" | "-v" => verbose = true,
            "--json" | "-j" => json = true,
            _ => {}
        }
        i += 1;
    }

    let path = input.unwrap_or_else(|| home_dir().join(".airgenome").join("vitals.jsonl"));
    let body = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => { eprintln!("cannot read {}: {}", path.display(), e); std::process::exit(1); }
    };

    let records = airgenome::parse_log(&body);
    if records.is_empty() {
        eprintln!("no valid records in {}", path.display());
        std::process::exit(1);
    }

    let mut engine = airgenome::PolicyEngine::with_defaults(capacity);
    let mut total_reactive = 0usize;
    let mut total_preempt = 0usize;
    let mut per_pair = [0usize; 15];
    let mut ticks_with_proposals = 0usize;

    for r in &records {
        let mut axes = [0.0; 6];
        axes[Axis::Cpu.index()] = r.cpu;
        axes[Axis::Ram.index()] = r.ram;
        axes[Axis::Gpu.index()] = r.gpu;
        axes[Axis::Npu.index()] = r.npu;
        axes[Axis::Power.index()] = r.power;
        axes[Axis::Io.index()] = r.io;
        let proposals = engine.tick(airgenome::Vitals { ts: r.ts, axes });

        if !proposals.is_empty() { ticks_with_proposals += 1; }
        for p in &proposals {
            per_pair[p.pair] += 1;
            match p.reason {
                airgenome::Reason::Reactive => total_reactive += 1,
                airgenome::Reason::Preemptive => total_preempt += 1,
            }
            if verbose {
                let tag = match p.reason {
                    airgenome::Reason::Reactive => "REACT ",
                    airgenome::Reason::Preemptive => "PREEMP",
                };
                println!("[{}] {} [{:>2}] {}", r.ts, tag, p.pair, p.action);
            }
        }
    }

    if json {
        print!("{{\"source\":\"{}\",\"records\":{},\"ticks_firing\":{},\"reactive\":{},\"preemptive\":{},\"per_pair\":[",
            path.display().to_string().replace('\\', "\\\\").replace('"', "\\\""),
            records.len(), ticks_with_proposals, total_reactive, total_preempt);
        for (k, n) in per_pair.iter().enumerate() {
            if k > 0 { print!(","); }
            print!("{}", n);
        }
        println!("]}}");
        return;
    }

    println!("=== airgenome — replay ===");
    println!("  Source       : {}", path.display());
    println!("  Records      : {}", records.len());
    println!("  Ticks firing : {} ({:.1}%)",
        ticks_with_proposals,
        100.0 * ticks_with_proposals as f64 / records.len() as f64);
    println!("  Reactive     : {}", total_reactive);
    println!("  Preemptive   : {}", total_preempt);
    println!("  Total        : {}", total_reactive + total_preempt);
    println!();
    println!("  Per-pair fire counts (sorted):");
    let mut pairs: Vec<(usize, usize)> = per_pair.iter().copied().enumerate().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));
    for (k, n) in &pairs {
        if *n == 0 { continue; }
        let (a, b) = airgenome::PAIRS[*k];
        println!("    [{:>2}] {:<6}×{:<6}  {:>5}", k, a.name(), b.name(), n);
    }
}

fn policy_cmd(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("watch");
    match sub {
        "watch" | "w" => policy_watch(args),
        "tick" | "t" => policy_tick_once(args),
        other => {
            eprintln!("unknown policy sub-command: '{}'", other);
            eprintln!("usage: airgenome policy [watch|tick] [-i SEC] [-c CAP]");
            std::process::exit(2);
        }
    }
}

fn policy_watch(args: &[String]) {
    let mut interval_s: u64 = 10;
    let mut capacity: usize = 12;
    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--interval" | "-i" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    interval_s = v.parse().unwrap_or(10).max(1);
                }
            }
            "--capacity" | "-c" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    capacity = v.parse().unwrap_or(12).max(3);
                }
            }
            _ => {}
        }
        i += 1;
    }

    let mut engine = airgenome::PolicyEngine::with_defaults(capacity);

    // Seed from recent vitals.jsonl if available (so preemptive fires fast).
    let log = home_dir().join(".airgenome").join("vitals.jsonl");
    if let Ok(body) = std::fs::read_to_string(&log) {
        let records = airgenome::parse_log(&body);
        let take = records.len().saturating_sub(capacity);
        for r in &records[take..] {
            let mut axes = [0.0; 6];
            axes[Axis::Cpu.index()] = r.cpu;
            axes[Axis::Ram.index()] = r.ram;
            axes[Axis::Gpu.index()] = r.gpu;
            axes[Axis::Npu.index()] = r.npu;
            axes[Axis::Power.index()] = r.power;
            axes[Axis::Io.index()] = r.io;
            engine.tick(airgenome::Vitals { ts: r.ts, axes });
        }
        eprintln!("seeded buffer with {} historical samples from {}",
            records[take..].len(), log.display());
    }

    eprintln!("airgenome policy watch — interval {}s, buffer cap {}", interval_s, capacity);
    eprintln!("Ctrl+C to stop.\n");

    loop {
        let v = airgenome::sample();
        let proposals = engine.tick(v);
        println!("[{}] {} proposals (cpu={:.2} ram={:.2} io={:.2})",
            v.ts, proposals.len(),
            v.get(Axis::Cpu), v.get(Axis::Ram), v.get(Axis::Io));
        for p in &proposals {
            let tag = match p.reason {
                airgenome::Reason::Reactive => "REACT ",
                airgenome::Reason::Preemptive => "PREEMP",
            };
            println!("  {} [{:>2}] {}", tag, p.pair, p.action);
        }
        std::thread::sleep(std::time::Duration::from_secs(interval_s));
    }
}

fn policy_tick_once(args: &[String]) {
    let json = args.iter().any(|a| a == "--json" || a == "-j");
    // Need ≥3 samples; seed from log, then add one fresh sample.
    let log = home_dir().join(".airgenome").join("vitals.jsonl");
    let body = match std::fs::read_to_string(&log) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {}: {}", log.display(), e);
            eprintln!("run `airgenome daemon` first to build a vitals log.");
            std::process::exit(1);
        }
    };
    let records = airgenome::parse_log(&body);
    if records.len() < 2 {
        eprintln!("need at least 2 samples in log; got {}", records.len());
        std::process::exit(1);
    }

    let mut engine = airgenome::PolicyEngine::with_defaults(12);
    let take = records.len().saturating_sub(11);
    for r in &records[take..] {
        let mut axes = [0.0; 6];
        axes[Axis::Cpu.index()] = r.cpu;
        axes[Axis::Ram.index()] = r.ram;
        axes[Axis::Gpu.index()] = r.gpu;
        axes[Axis::Npu.index()] = r.npu;
        axes[Axis::Power.index()] = r.power;
        axes[Axis::Io.index()] = r.io;
        engine.tick(airgenome::Vitals { ts: r.ts, axes });
    }
    let v = airgenome::sample();
    let proposals = engine.tick(v);

    if json {
        print!("{{\"ts\":{},\"cpu\":{},\"ram\":{},\"io\":{},\"proposals\":[",
            v.ts, v.get(Axis::Cpu), v.get(Axis::Ram), v.get(Axis::Io));
        for (i, p) in proposals.iter().enumerate() {
            if i > 0 { print!(","); }
            let reason = match p.reason {
                airgenome::Reason::Reactive => "reactive",
                airgenome::Reason::Preemptive => "preemptive",
            };
            print!("{{\"pair\":{},\"reason\":\"{}\"}}", p.pair, reason);
        }
        println!("]}}");
        return;
    }

    println!("=== airgenome — policy tick ===");
    println!("  ts={}  cpu={:.2} ram={:.2} io={:.2}",
        v.ts, v.get(Axis::Cpu), v.get(Axis::Ram), v.get(Axis::Io));
    println!("  {} proposals:", proposals.len());
    for p in &proposals {
        let tag = match p.reason {
            airgenome::Reason::Reactive => "REACT ",
            airgenome::Reason::Preemptive => "PREEMP",
        };
        println!("    {} [{:>2}] {}", tag, p.pair, p.action);
    }
}

fn summary_cmd() {
    // Compact overview: version, current vitals, firing, trace stats.
    let v = airgenome::sample();
    let firing = airgenome::firing(&v);
    let work = 1.0 - (firing.len() as f64) / (PAIR_COUNT as f64);

    println!("airgenome {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Now (ts={}):", v.ts);
    println!("  cpu={:.2} ram={:.2} io={:.2} power={:.0}  |  firing {}/{}  |  work {:.3}",
        v.get(Axis::Cpu), v.get(Axis::Ram), v.get(Axis::Io), v.get(Axis::Power),
        firing.len(), PAIR_COUNT, work);

    // Trace stats from log.
    let log = home_dir().join(".airgenome").join("vitals.jsonl");
    match std::fs::read_to_string(&log) {
        Ok(body) => {
            let records = airgenome::parse_log(&body);
            let stats = airgenome::summarize(&records);
            if stats.count > 0 {
                let hours = stats.span_secs as f64 / 3600.0;
                println!();
                println!("Log ({:.1} h, {} samples):", hours, stats.count);
                println!("  mean cpu={:.2} ram={:.2} io={:.2}  |  firing μ={:.1} max={}  |  work {:.3}",
                    stats.cpu_mean, stats.ram_mean, stats.io_mean,
                    stats.firing_mean, stats.firing_max, stats.work_fraction);
                if stats.on_battery_frac > 0.0 {
                    println!("  on battery {:.1}%", stats.on_battery_frac * 100.0);
                }
            }
        }
        Err(_) => {
            println!();
            println!("Log: (not yet written — run `airgenome daemon` or `init`)");
        }
    }

    // Daemon health.
    let loaded = std::process::Command::new("launchctl")
        .arg("list").arg("com.airgenome.daemon").output()
        .map(|o| o.status.success()).unwrap_or(false);
    println!();
    println!("Daemon: {}", if loaded { green("running") } else { yellow("not loaded") });
}

fn doctor_cmd() {
    let mut pass = 0usize;
    let mut warn = 0usize;
    let mut fail = 0usize;

    let ok = |label: &str, detail: &str| println!("  [{}] {:<22} {}", green("ok  "), label, detail);
    let wrn = |label: &str, detail: &str| println!("  [{}] {:<22} {}", yellow("warn"), label, detail);
    let bad = |label: &str, detail: &str| println!("  [{}] {:<22} {}", red("fail"), label, dim(detail));

    println!("=== airgenome — doctor ===");
    println!();

    // 1. binary path
    match std::env::current_exe() {
        Ok(p) => { ok("binary", &p.display().to_string()); pass += 1; }
        Err(e) => { bad("binary", &format!("{}", e)); fail += 1; }
    }

    // 2. data dir
    let data_dir = home_dir().join(".airgenome");
    if data_dir.exists() {
        ok("data dir", &data_dir.display().to_string());
        pass += 1;
    } else {
        wrn("data dir", "missing (run `airgenome daemon` or `init`)");
        warn += 1;
    }

    // 3. LaunchAgent plist
    let plist = home_dir().join("Library/LaunchAgents/com.airgenome.daemon.plist");
    if plist.exists() {
        ok("LaunchAgent", &plist.display().to_string());
        pass += 1;
    } else {
        wrn("LaunchAgent", "not registered (run `airgenome init`)");
        warn += 1;
    }

    // 4. LaunchAgent loaded
    let loaded = std::process::Command::new("launchctl")
        .arg("list").arg("com.airgenome.daemon").output()
        .map(|o| o.status.success()).unwrap_or(false);
    if loaded {
        ok("agent loaded", "com.airgenome.daemon");
        pass += 1;
    } else {
        wrn("agent loaded", "not running (launchctl list says no)");
        warn += 1;
    }

    // 5. vitals.jsonl freshness
    let log = data_dir.join("vitals.jsonl");
    if let Ok(meta) = std::fs::metadata(&log) {
        let age_s = meta.modified().ok()
            .and_then(|t| t.elapsed().ok())
            .map(|d| d.as_secs())
            .unwrap_or(u64::MAX);
        if age_s < 180 {
            ok("vitals log", &format!("fresh ({}s old, {} bytes)", age_s, meta.len()));
            pass += 1;
        } else if age_s < 3600 {
            wrn("vitals log", &format!("{}s old — daemon may be stopped", age_s));
            warn += 1;
        } else {
            bad("vitals log", &format!("stale ({} min old)", age_s / 60));
            fail += 1;
        }
    } else {
        wrn("vitals log", "not written yet");
        warn += 1;
    }

    // 6. active genome
    let active = data_dir.join("active.genome");
    if let Ok(bytes) = std::fs::read(&active) {
        if bytes.len() == GENOME_BYTES {
            let engaged = bytes.chunks(4)
                .filter(|c| c.iter().any(|&b| b != 0)).count();
            ok("active profile", &format!("{} pairs engaged", engaged));
            pass += 1;
        } else {
            bad("active profile", &format!("bad size {} (expected {})",
                bytes.len(), GENOME_BYTES));
            fail += 1;
        }
    } else {
        wrn("active profile", "none set (run `airgenome profile apply <name>`)");
        warn += 1;
    }

    println!();
    println!("Summary: {} pass · {} warn · {} fail", pass, warn, fail);
    if fail > 0 {
        std::process::exit(1);
    }
}

fn init_cmd(args: &[String]) {
    // Parse --interval N (seconds).
    let mut interval_s: u64 = 60;
    let mut i = 2;
    while i < args.len() {
        if args[i] == "--interval" || args[i] == "-i" {
            i += 1;
            if let Some(v) = args.get(i) { interval_s = v.parse().unwrap_or(60).max(1); }
        }
        i += 1;
    }

    let bin = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => { eprintln!("cannot resolve current binary: {}", e); std::process::exit(1); }
    };
    let data_dir = home_dir().join(".airgenome");
    let agents_dir = home_dir().join("Library/LaunchAgents");
    let plist = agents_dir.join("com.airgenome.daemon.plist");

    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        eprintln!("cannot create {}: {}", data_dir.display(), e);
        std::process::exit(1);
    }
    if let Err(e) = std::fs::create_dir_all(&agents_dir) {
        eprintln!("cannot create {}: {}", agents_dir.display(), e);
        std::process::exit(1);
    }

    let contents = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.airgenome.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>{bin}</string>
        <string>daemon</string>
        <string>--interval</string>
        <string>{interval}</string>
        <string>--output</string>
        <string>{data}/vitals.jsonl</string>
    </array>
    <key>RunAtLoad</key><true/>
    <key>KeepAlive</key><true/>
    <key>StandardOutPath</key><string>{data}/daemon.out.log</string>
    <key>StandardErrorPath</key><string>{data}/daemon.err.log</string>
    <key>WorkingDirectory</key><string>{home}</string>
    <key>ProcessType</key><string>Background</string>
    <key>LowPriorityIO</key><true/>
    <key>Nice</key><integer>10</integer>
</dict>
</plist>
"#,
        bin = bin.display(),
        interval = interval_s,
        data = data_dir.display(),
        home = home_dir().display(),
    );

    if let Err(e) = std::fs::write(&plist, contents) {
        eprintln!("cannot write {}: {}", plist.display(), e);
        std::process::exit(1);
    }

    // reload launchd
    let _ = std::process::Command::new("launchctl")
        .args(["unload", &plist.to_string_lossy()])
        .status();
    let load = std::process::Command::new("launchctl")
        .args(["load", &plist.to_string_lossy()])
        .status();

    match load {
        Ok(s) if s.success() => {
            println!("init: LaunchAgent loaded ({}s interval)", interval_s);
            println!("  plist : {}", plist.display());
            println!("  bin   : {}", bin.display());
            println!("  data  : {}", data_dir.display());
            println!();
            println!("run `airgenome policy watch` to see it live.");
        }
        _ => {
            eprintln!("launchctl load failed; see {}/daemon.err.log", data_dir.display());
            std::process::exit(1);
        }
    }
}

fn uninit_cmd() {
    let plist = home_dir().join("Library/LaunchAgents/com.airgenome.daemon.plist");
    let data_dir = home_dir().join(".airgenome");

    if plist.exists() {
        let _ = std::process::Command::new("launchctl")
            .args(["unload", &plist.to_string_lossy()])
            .status();
        if let Err(e) = std::fs::remove_file(&plist) {
            eprintln!("could not remove {}: {}", plist.display(), e);
        } else {
            println!("uninit: removed {}", plist.display());
        }
    } else {
        println!("uninit: no LaunchAgent at {}", plist.display());
    }
    println!("  data preserved: {}", data_dir.display());
    println!("  rm -rf {}  # to wipe collected vitals", data_dir.display());
}

fn home_dir() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
}

fn print_help() {
    println!("airgenome — 6-axis Mac Air resource hexagon\n");
    println!("USAGE:");
    println!("  airgenome [SUBCOMMAND]\n");
    println!("SUBCOMMANDS:");
    println!("  status              show hexagon + vitals + firing count (default)");
    println!("  probe               emit a single JSON vitals sample");
    println!("  sample -n N -i SEC  emit JSON array of N vitals samples");
    println!("  pairs               list the 15 canonical pair gates");
    println!("  rules               list the 15 rules with mesh neighbors");
    println!("  diag                fire rules on current vitals + dry-run proposals");
    println!("  dash [--watch -i N] ascii hexagon dashboard (axes + 15 pair gates)");
    println!("  metrics             Prometheus text-format exposition");
    println!("  explain K           explain pair gate K (0..14) + current state");
    println!("  action [K] [--no-sudo|--sudo-only]  concrete shell commands per firing pair");
    println!("  plan                Tier 1 UserAction plan per firing pair (dry-run)");
    println!("  apply K [--yes|--confirm] [--measure]     execute Tier 1 action for pair K");
    println!("  apply-all [--yes] [--measure]             apply every firing pair in one pass");
    println!("  profile list        list built-in profiles");
    println!("  profile show NAME   show engaged pairs + genome hex");
    println!("  profile apply NAME  apply built-in/user profile OR .genome file path");
    println!("  profile active      show the currently applied profile");
    println!("  diff A B            show per-pair genome differences between profiles");
    println!("  genome save|cat|hex genome file I/O (save/load 60-byte binary .genome)");
    println!("  daemon [-i SEC]     periodic vitals loop → ~/.airgenome/vitals.jsonl");
    println!("  daemon --once       append one sample + exit (for cron)");
    println!("  trace [--tail N]    summarize ~/.airgenome/vitals.jsonl");
    println!("  replay [-v]         replay log through PolicyEngine, tally fires");
    println!("  export -f csv|json  export vitals.jsonl as CSV or JSON array");
    println!("  policy watch        live PolicyEngine loop (preemptive + reactive)");
    println!("  policy tick         one-shot: seed from log + evaluate current vitals");
    println!("  init [-i SEC]       register LaunchAgent so the daemon auto-starts");
    println!("  uninit              unload + remove the LaunchAgent");
    println!("  doctor              self-diagnostic (binary/agent/log/profile)");
    println!("  summary             compact overview: now + log stats + daemon status");
    println!("  version             print airgenome version");
    println!("  help                print this help");
}
