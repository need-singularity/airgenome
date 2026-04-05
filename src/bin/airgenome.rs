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
        "simulate" | "sim" => simulate_cmd(&args),
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
        "helper" => helper_cmd(&args),
        "purge" => purge_cmd(&args),
        "tune" => tune_cmd(&args),
        "sysctl" => sysctl_cmd(&args),
        "reap" => reap_cmd(&args),
        "coverage" | "cov" => coverage_cmd(),
        "insights" | "ins" => insights_cmd(&args),
        "idle-capacity" | "idle" => idle_capacity_cmd(),
        "transitions" | "trans" => transitions_cmd(&args),
        "anomalies" | "anom" => anomalies_cmd(&args),
        "processes" | "proc" => processes_cmd(),
        "signature" | "sig" => signature_cmd(&args),
        "signature-history" | "sig-hist" => signature_history_cmd(&args),
        "fingerprints" | "fp" => fingerprints_cmd(),
        "fingerprint-save" | "fp-save" => fingerprint_save_cmd(&args),
        "match" => match_cmd(&args),
        "match-distribution" | "match-dist" => match_distribution_cmd(),
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

fn simulate_cmd(args: &[String]) {
    let scenario = args.get(2).map(|s| s.as_str()).unwrap_or("help");
    let (label, vitals) = match scenario {
        "ram-pressure" => ("RAM pressure 95%, CPU high, on AC",
            airgenome::Vitals {
                ts: 0,
                axes: [5.5, 0.95, 8.0, 8.0, 1.0, 2.5],
            }),
        "thermal-throttle" => ("CPU maxed, battery, IO spike",
            airgenome::Vitals {
                ts: 0,
                axes: [7.8, 0.6, 8.0, 8.0, 0.0, 3.0],
            }),
        "battery-drain" => ("battery mode, light load, swap active",
            airgenome::Vitals {
                ts: 0,
                axes: [2.0, 0.75, 8.0, 8.0, 0.0, 1.5],
            }),
        "ml-inference" => ("GPU/NPU active, ram tight, AC",
            airgenome::Vitals {
                ts: 0,
                axes: [4.0, 0.88, 8.0, 8.0, 1.0, 1.2],
            }),
        "idle" => ("idle baseline",
            airgenome::Vitals {
                ts: 0,
                axes: [0.3, 0.15, 8.0, 8.0, 1.0, 0.5],
            }),
        _ => {
            println!("usage: airgenome simulate <scenario>");
            println!("scenarios:");
            println!("  ram-pressure     high RAM + high CPU on AC");
            println!("  thermal-throttle CPU maxed on battery");
            println!("  battery-drain    swap + battery mode");
            println!("  ml-inference     GPU/NPU active, RAM tight");
            println!("  idle             light load baseline");
            std::process::exit(2);
        }
    };

    println!("=== simulate: {} ({}) ===", scenario, label);
    println!();
    println!("synthetic vitals:");
    for axis in Axis::ALL {
        println!("  {:<6} = {:.3}", axis.name(), vitals.get(axis));
    }
    println!();

    let firing = airgenome::firing(&vitals);
    println!("rules firing: {}/{}", firing.len(), PAIR_COUNT);
    for &k in &firing {
        let (a, b) = PAIRS[k];
        let sev = airgenome::severity(k, &vitals);
        let tag = match sev {
            airgenome::Severity::Ok => dim("ok"),
            airgenome::Severity::Warn => yellow("warn"),
            airgenome::Severity::Critical => red("CRITICAL"),
        };
        println!("  [{:>2}] {}×{}  {}", k, a.name(), b.name(), tag);
    }
    println!();

    println!("Tier 1 plan:");
    let mut planned = 0;
    for &k in &firing {
        if let Some(action) = airgenome::plan_for_pair(k) {
            planned += 1;
            println!("  [{:>2}] {}", k, action.label());
        }
    }
    println!("  → {} Tier 1 actions / {} firing pairs", planned, firing.len());
    let work = 1.0 - (firing.len() as f64) / (PAIR_COUNT as f64);
    println!();
    println!("work fraction: {:.3}  (ceiling {:.3})", work, airgenome::WORK_FP);
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

fn fingerprints_cmd() {
    println!("Built-in workload fingerprints ({}):", airgenome::signature::FINGERPRINTS.len());
    println!();
    for fp in airgenome::signature::FINGERPRINTS {
        println!("  {:<14} {}", fp.name, fp.description);
        print!("    [");
        for (i, v) in fp.signature.axes.iter().enumerate() {
            if i > 0 { print!(","); }
            print!("{:.2}", v);
        }
        println!("]");
    }

    // List custom fingerprints.
    let path = home_dir().join(".airgenome").join("custom_fingerprints.jsonl");
    if let Ok(body) = std::fs::read_to_string(&path) {
        let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
        if !lines.is_empty() {
            println!();
            println!("Custom fingerprints ({}):  {}", lines.len(), path.display());
            for line in lines {
                // naive parse: pull "name" and "axes" fields
                let name = line.split("\"name\":\"").nth(1)
                    .and_then(|s| s.split('"').next()).unwrap_or("?");
                println!("  {}", name);
            }
        }
    }
}

fn fingerprint_save_cmd(args: &[String]) {
    let Some(name) = args.get(2) else {
        eprintln!("usage: airgenome fingerprint-save <name>");
        std::process::exit(2);
    };
    if name.chars().any(|c| c == '"' || c == '\\' || c == '\n') {
        eprintln!("invalid characters in name");
        std::process::exit(2);
    }
    let v = airgenome::sample();
    let path = home_dir().join(".airgenome").join("custom_fingerprints.jsonl");
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    let mut f = match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => f,
        Err(e) => { eprintln!("cannot open {}: {}", path.display(), e); std::process::exit(1); }
    };
    use std::io::Write as _;
    let _ = writeln!(f,
        "{{\"ts\":{},\"name\":\"{}\",\"axes\":[{},{},{},{},{},{}]}}",
        v.ts, name,
        v.axes[0], v.axes[1], v.axes[2], v.axes[3], v.axes[4], v.axes[5]);
    println!("saved fingerprint '{}' at ts={}", name, v.ts);
    println!("  axes: {:?}", v.axes);
}

fn load_custom_fingerprints() -> Vec<(String, airgenome::signature::Signature)> {
    let path = home_dir().join(".airgenome").join("custom_fingerprints.jsonl");
    let body = match std::fs::read_to_string(&path) { Ok(s) => s, Err(_) => return Vec::new() };
    let mut out = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let name = line.split("\"name\":\"").nth(1)
            .and_then(|s| s.split('"').next()).unwrap_or("");
        if name.is_empty() { continue; }
        let axes_str = match line.split("\"axes\":[").nth(1)
            .and_then(|s| s.split(']').next()) { Some(s) => s, None => continue };
        let axes: Vec<f64> = axes_str.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if axes.len() == 6 {
            let a: [f64; 6] = [axes[0], axes[1], axes[2], axes[3], axes[4], axes[5]];
            out.push((name.to_string(), airgenome::signature::Signature::new(a)));
        }
    }
    out
}

fn match_cmd(args: &[String]) {
    let append = args.iter().any(|a| a == "--append");
    let json = args.iter().any(|a| a == "--json" || a == "-j");

    let v = airgenome::sample();
    let sig = airgenome::signature::Signature::new(v.axes);

    // Combine built-in + custom.
    let mut all: Vec<(String, airgenome::signature::Signature, &'static str)> =
        airgenome::signature::FINGERPRINTS.iter()
            .map(|fp| (fp.name.to_string(), fp.signature, "built-in"))
            .collect();
    for (n, s) in load_custom_fingerprints() {
        all.push((n, s, "custom"));
    }

    let mut scored: Vec<_> = all.iter()
        .map(|(n, s, k)| (n.clone(), s.clone(), *k, sig.euclidean(s)))
        .collect();
    scored.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal));
    let (best_name, best_sig, best_kind, dist) = scored.first().unwrap().clone();
    let cos = sig.cosine(&best_sig);
    let _ = best_kind; // silence unused in some paths

    if append {
        let path = home_dir().join(".airgenome").join("matches.jsonl");
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(f,
                "{{\"ts\":{},\"match\":\"{}\",\"kind\":\"{}\",\"distance\":{:.3},\"cosine\":{:.3}}}",
                v.ts, best_name, best_kind, dist, cos);
        }
        println!("appended: ts={} match={} ({}) d={:.3}", v.ts, best_name, best_kind, dist);
        return;
    }

    if json {
        println!("{{\"ts\":{},\"match\":\"{}\",\"kind\":\"{}\",\"distance\":{:.3},\"cosine\":{:.3}}}",
            v.ts, best_name, best_kind, dist, cos);
        return;
    }

    println!("=== airgenome — match current vitals to workload fingerprint ===");
    println!();
    print!("  current: [");
    for (i, a) in sig.axes.iter().enumerate() {
        if i > 0 { print!(","); }
        print!("{:.2}", a);
    }
    println!("]");
    println!();
    println!("  nearest: {} ({})  d={:.3}  cos={:.3}",
        green(&best_name), best_kind, dist, cos);
    println!();
    println!("  ranked (top 6):");
    for (name, _s, kind, d) in scored.iter().take(6) {
        let tag = if *kind == "custom" { yellow("custom") } else { dim("built-in") };
        println!("    {:<18} {}  d={:.3}", name, tag, d);
    }
}

fn match_distribution_cmd() {
    let path = home_dir().join(".airgenome").join("matches.jsonl");
    let body = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {}: {}", path.display(), e);
            eprintln!("hint: run `airgenome match --append` first (or crontab)");
            std::process::exit(1);
        }
    };

    fn f<'a>(s: &'a str, key: &str) -> Option<&'a str> {
        let needle = format!("\"{}\"", key);
        let i = s.find(&needle)?;
        let rest = &s[i + needle.len()..];
        let colon = rest.find(':')?;
        let after = rest[colon + 1..].trim_start();
        if let Some(stripped) = after.strip_prefix('"') {
            let end = stripped.find('"')?;
            Some(&stripped[..end])
        } else {
            let end = after.find(|c: char| c == ',' || c == '}').unwrap_or(after.len());
            Some(after[..end].trim())
        }
    }

    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut dsum: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
    let mut total = 0;
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        total += 1;
        let name = match f(line, "match") { Some(n) => n.to_string(), None => continue };
        let dist: f64 = f(line, "distance").and_then(|s| s.parse().ok()).unwrap_or(0.0);
        *counts.entry(name.clone()).or_insert(0) += 1;
        *dsum.entry(name).or_insert(0.0) += dist;
    }

    println!("=== airgenome — workload distribution ({} matches) ===", total);
    println!();
    println!("  fingerprint    count    %      mean d");
    println!("  ────────────────────────────────────────");
    let mut rows: Vec<_> = counts.iter().collect();
    rows.sort_by(|a,b| b.1.cmp(a.1));
    for (name, n) in rows {
        let pct = 100.0 * (*n as f64) / (total as f64);
        let mean_d = dsum.get(name).unwrap_or(&0.0) / (*n as f64);
        println!("  {:<14}  {:>5}  {:>5.1}%  {:>6.2}", name, n, pct, mean_d);
    }
}

fn signature_history_cmd(args: &[String]) {
    let filter = args.get(2).cloned();
    let path = home_dir().join(".airgenome").join("signatures.jsonl");
    let body = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {}: {}", path.display(), e);
            eprintln!("hint: run `airgenome signature --append` first");
            std::process::exit(1);
        }
    };

    // Parse rows with our flat JSON reader.
    fn f<'a>(s: &'a str, key: &str) -> Option<&'a str> {
        let needle = format!("\"{}\"", key);
        let i = s.find(&needle)?;
        let rest = &s[i + needle.len()..];
        let colon = rest.find(':')?;
        let after = rest[colon + 1..].trim_start();
        if let Some(stripped) = after.strip_prefix('"') {
            let end = stripped.find('"')?;
            Some(&stripped[..end])
        } else {
            let end = after.find(|c: char| c == ',' || c == '}').unwrap_or(after.len());
            Some(after[..end].trim())
        }
    }

    #[derive(Default, Clone)]
    struct Stat { n: usize, rss_sum: f64, rss_max: f64, cpu_sum: f64, cpu_max: f64, ts_first: u64, ts_last: u64 }
    let mut stats: std::collections::BTreeMap<String, Stat> = std::collections::BTreeMap::new();
    let mut total_rows = 0usize;

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        total_rows += 1;
        let cat = match f(line, "category") { Some(c) => c.to_string(), None => continue };
        if let Some(q) = &filter { if &cat != q { continue; } }
        let rss: f64 = f(line, "rss_pct").and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let cpu: f64 = f(line, "cpu_pct").and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let ts: u64 = f(line, "ts").and_then(|s| s.parse().ok()).unwrap_or(0);

        let s = stats.entry(cat).or_default();
        s.n += 1;
        s.rss_sum += rss;
        if rss > s.rss_max { s.rss_max = rss; }
        s.cpu_sum += cpu;
        if cpu > s.cpu_max { s.cpu_max = cpu; }
        if s.ts_first == 0 || ts < s.ts_first { s.ts_first = ts; }
        if ts > s.ts_last { s.ts_last = ts; }
    }

    println!("=== airgenome — signature history ({} rows) ===", total_rows);
    println!();
    println!("  category     n   rss%_μ  rss%_max  cpu%_μ  cpu%_max  span");
    println!("  ──────────────────────────────────────────────────────────");
    let mut rows: Vec<_> = stats.iter().collect();
    rows.sort_by(|a,b| b.1.rss_sum.partial_cmp(&a.1.rss_sum).unwrap_or(std::cmp::Ordering::Equal));
    for (cat, s) in rows {
        let n = s.n as f64;
        let rss_mu = s.rss_sum / n;
        let cpu_mu = s.cpu_sum / n;
        let span_h = (s.ts_last - s.ts_first) as f64 / 3600.0;
        println!("  {:<10}  {:>4}  {:>5.1}%  {:>7.1}%  {:>5.1}  {:>7.1}   {:.1}h",
            cat, s.n, rss_mu * 100.0, s.rss_max * 100.0, cpu_mu, s.cpu_max, span_h);
    }
}

fn signature_cmd(args: &[String]) {
    // Project each process category onto the 6-axis hexagon and report
    // which firing rules the category *would* trigger if it were the
    // sole load on the system.
    let filter = args.iter().skip(2).find(|a| !a.starts_with('-')).cloned();
    let append = args.iter().any(|a| a == "--append");
    let json = args.iter().any(|a| a == "--json" || a == "-j");

    let output = match std::process::Command::new("ps")
        .args(["-axm", "-o", "rss=,pcpu=,comm="])
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => { eprintln!("ps failed"); std::process::exit(1); }
    };

    // total RAM (pages × 16 KB / 1 MB).
    let total_ram_mb: f64 = {
        let out = std::process::Command::new("sysctl").args(["-n", "hw.memsize"]).output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        out.parse::<f64>().map(|b| b / 1024.0 / 1024.0).unwrap_or(8192.0)
    };

    let categorize = |path: &str| -> &'static str {
        let l = path.to_lowercase();
        if l.contains("chrome") || l.contains("safari") || l.contains("firefox") || l.contains("arc")
           || l.contains("webkit") { "browser" }
        else if l.contains("slack") || l.contains("discord") || l.contains("telegram") { "im" }
        else if l.contains("vscode") || l.contains("code helper") || l.contains("cursor")
             || l.contains("zed") { "ide" }
        else if l.contains("terminal") || l.contains("iterm") || l.contains("warp") { "terminal" }
        else if l.contains("rustc") || l.contains("cargo") { "rust" }
        else if l.contains("python") { "python" }
        else if l.contains("node") { "node" }
        else if l.contains("finder") { "finder" }
        else if l.contains("docker") || l.contains("orb") { "container" }
        else if l.contains("ollama") { "llm" }
        else if l.contains("windowserver") || l.contains("launchd") || l.contains("mdworker") { "system" }
        else { "other" }
    };

    #[derive(Default, Clone)]
    struct Cat { rss_mb: f64, cpu_pct: f64, procs: usize }
    let mut cats: std::collections::BTreeMap<&'static str, Cat> = std::collections::BTreeMap::new();

    for line in output.lines() {
        let t = line.trim();
        if t.is_empty() { continue; }
        let mut it = t.split_whitespace();
        let rss_kb: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let cpu: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let comm = it.collect::<Vec<_>>().join(" ");
        if comm.is_empty() { continue; }
        let cat = categorize(&comm);
        let c = cats.entry(cat).or_default();
        c.rss_mb += rss_kb / 1024.0;
        c.cpu_pct += cpu;
        c.procs += 1;
    }

    // Current timestamp for logging.
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs()).unwrap_or(0);

    // Open signature log if --append.
    let mut log_file: Option<std::fs::File> = if append {
        let path = home_dir().join(".airgenome").join("signatures.jsonl");
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        std::fs::OpenOptions::new().create(true).append(true).open(&path).ok()
    } else { None };

    if !json && !append {
        println!("=== airgenome — hexagon signature per category ===");
        println!("  (system total RAM: {:.0} MB; projection: cpu→load-like, ram→rss share)", total_ram_mb);
        println!();
        println!("  category     procs   rss%    cpu    firing  pairs");
        println!("  ────────────────────────────────────────────────────");
    }

    let mut json_rows: Vec<String> = Vec::new();

    // Gate: treat rss%/total_ram as "ram" axis [0..1], cpu_pct/100 as cpu load.
    // gpu/npu/power/io axes are unknown per-category — leave 0.0 except
    // power=1 (AC default). This is a simplified projection; the full
    // per-source 6-axis signature is future work.
    let mut entries: Vec<(&&str, Cat)> = cats.iter().map(|(k,v)| (k, v.clone())).collect();
    entries.sort_by(|a,b| b.1.rss_mb.partial_cmp(&a.1.rss_mb).unwrap_or(std::cmp::Ordering::Equal));

    for (name, c) in &entries {
        if let Some(f) = &filter { if **name != f.as_str() { continue; } }
        let rss_frac = (c.rss_mb / total_ram_mb).clamp(0.0, 1.0);
        let cpu_load = (c.cpu_pct / 100.0).max(0.0);
        let axes = [cpu_load, rss_frac, 0.0, 0.0, 1.0, 0.0];
        let v = airgenome::Vitals { ts: 0, axes };
        let fires: Vec<usize> = airgenome::firing(&v);
        let fires_str = if fires.is_empty() { "·".to_string() }
                        else { fires.iter().map(|k| k.to_string()).collect::<Vec<_>>().join(",") };

        let row = format!(
            "{{\"ts\":{},\"category\":\"{}\",\"procs\":{},\"rss_mb\":{:.1},\"rss_pct\":{:.3},\"cpu_pct\":{:.1},\"firing\":{}}}",
            ts, name, c.procs, c.rss_mb, rss_frac, c.cpu_pct, fires.len());

        if let Some(f) = log_file.as_mut() {
            use std::io::Write as _;
            let _ = writeln!(f, "{}", row);
        }

        if json {
            json_rows.push(row);
        } else if !append {
            println!("  {:<11}  {:>5}  {:>5.1}%  {:>5.1}  {:>5}   [{}]",
                name, c.procs, rss_frac * 100.0, c.cpu_pct, fires.len(), fires_str);
        }
    }

    if json {
        print!("[");
        for (i, row) in json_rows.iter().enumerate() {
            if i > 0 { print!(","); }
            print!("{}", row);
        }
        println!("]");
    } else if append {
        println!("appended {} rows to ~/.airgenome/signatures.jsonl", entries.len());
    } else {
        println!();
        println!("  {}", dim("firing interpretation: if this category were the ONLY load,"));
        println!("  {}", dim("these pair gates would fire (ignoring gpu/npu/power/io unknowns)."));
    }
}

fn processes_cmd() {
    // Read `ps -axm -o rss,pcpu,comm` (sorted by RSS descending).
    let output = match std::process::Command::new("ps")
        .args(["-axm", "-o", "rss=,pcpu=,comm="])
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => { eprintln!("ps failed"); std::process::exit(1); }
    };

    #[derive(Default)]
    struct Bucket { procs: usize, rss_kb: u64, cpu: f64 }
    let mut buckets: std::collections::BTreeMap<&'static str, Bucket> = std::collections::BTreeMap::new();
    let categorize = |path: &str| -> &'static str {
        let l = path.to_lowercase();
        if l.contains("chrome") || l.contains("safari") || l.contains("firefox") || l.contains("arc")
           || l.contains("webkit") { "browser" }
        else if l.contains("slack") || l.contains("discord") || l.contains("telegram")
             || l.contains("whatsapp") { "im" }
        else if l.contains("vscode") || l.contains("code helper") || l.contains("cursor")
             || l.contains("zed") || l.contains("xcode") { "ide" }
        else if l.contains("terminal") || l.contains("iterm") || l.contains("warp")
             || l.contains("alacritty") { "terminal" }
        else if l.contains("rustc") || l.contains("cargo") { "rust" }
        else if l.contains("python") { "python" }
        else if l.contains("node") { "node" }
        else if l.contains("java") || l.contains("jvm") { "java" }
        else if l.contains("finder") { "finder" }
        else if l.contains("docker") || l.contains("orb") { "container" }
        else if l.contains("ollama") || l.contains("llama") { "llm" }
        else if l.starts_with("/system/") || l.contains("windowserver") || l.contains("coreaudio")
             || l.contains("launchd") || l.contains("mdworker") || l.contains("cfprefs") { "system" }
        else { "other" }
    };

    let mut rows: Vec<(u64, f64, String)> = Vec::new();
    for line in output.lines().skip(0) {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        let mut it = trimmed.split_whitespace();
        let rss_kb: u64 = match it.next().and_then(|s| s.parse().ok()) { Some(v) => v, None => continue };
        let cpu: f64 = match it.next().and_then(|s| s.parse().ok()) { Some(v) => v, None => continue };
        let comm: String = it.collect::<Vec<_>>().join(" ");
        if comm.is_empty() { continue; }
        rows.push((rss_kb, cpu, comm));
    }

    // Bucket aggregate.
    for (rss_kb, cpu, comm) in &rows {
        let cat = categorize(comm);
        let b = buckets.entry(cat).or_default();
        b.procs += 1;
        b.rss_kb += rss_kb;
        b.cpu += cpu;
    }

    println!("=== airgenome — processes ({} total) ===", rows.len());
    println!();
    println!("  Category     procs     RSS     CPU%");
    println!("  ─────────────────────────────────────");
    let mut bv: Vec<_> = buckets.iter().collect();
    bv.sort_by(|a,b| b.1.rss_kb.cmp(&a.1.rss_kb));
    for (cat, b) in &bv {
        let mb = b.rss_kb / 1024;
        let mb_str = if mb >= 1024 { format!("{:.1}GB", mb as f64 / 1024.0) }
                     else { format!("{}MB", mb) };
        println!("  {:<10}  {:>5}  {:>7}  {:>6.1}", cat, b.procs, mb_str, b.cpu);
    }

    println!();
    println!("  Top 10 RSS:");
    rows.sort_by(|a,b| b.0.cmp(&a.0));
    for (rss_kb, cpu, comm) in rows.iter().take(10) {
        let mb = rss_kb / 1024;
        let name = comm.split('/').next_back().unwrap_or(comm);
        let name_short: String = name.chars().take(40).collect();
        println!("    {:>6} MB  {:>5.1}%  {}", mb, cpu, name_short);
    }
}

fn anomalies_cmd(args: &[String]) {
    // For each vitals sample, find distance to nearest built-in+custom fingerprint.
    // Flag samples where min distance > threshold (default = 10.0).
    let threshold: f64 = args.iter().position(|a| a == "--threshold" || a == "-t")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(10.0);

    let log = home_dir().join(".airgenome").join("vitals.jsonl");
    let body = match std::fs::read_to_string(&log) {
        Ok(s) => s,
        Err(e) => { eprintln!("cannot read {}: {}", log.display(), e); std::process::exit(1); }
    };
    let records = airgenome::parse_log(&body);

    // Combine built-in + custom.
    let mut all: Vec<(String, airgenome::signature::Signature)> =
        airgenome::signature::FINGERPRINTS.iter()
            .map(|fp| (fp.name.to_string(), fp.signature)).collect();
    for (n, s) in load_custom_fingerprints() { all.push((n, s)); }

    let mut outliers = 0usize;
    let mut max_d = 0.0f64;
    let mut max_ts = 0u64;
    let mut distances: Vec<f64> = Vec::with_capacity(records.len());

    println!("=== airgenome — anomalies ({} samples, threshold d>{:.1}) ===",
        records.len(), threshold);
    println!();
    println!("  ts          d_min   nearest           axes");
    println!("  ──────────────────────────────────────────────────────");
    let mut shown = 0;
    for r in &records {
        let sig = airgenome::signature::Signature::new(
            [r.cpu, r.ram, r.gpu, r.npu, r.power, r.io]);
        let (_, best_d, best_name) = all.iter()
            .map(|(n, s)| (n.clone(), sig.euclidean(s), n.as_str()))
            .fold((String::new(), f64::INFINITY, ""),
                |acc, (n, d, nm)| if d < acc.1 { (n, d, nm) } else { acc });
        distances.push(best_d);
        if best_d > max_d { max_d = best_d; max_ts = r.ts; }
        if best_d > threshold {
            outliers += 1;
            if shown < 10 {
                println!("  {}  {:>5.2}  {:<16}  [{:.1},{:.2},{:.0},{:.0},{:.0},{:.2}]",
                    r.ts, best_d, best_name, r.cpu, r.ram, r.gpu, r.npu, r.power, r.io);
                shown += 1;
            }
        }
    }
    if outliers == 0 { println!("  (none above threshold)"); }

    let mean_d: f64 = distances.iter().sum::<f64>() / distances.len().max(1) as f64;
    println!();
    println!("Summary:");
    println!("  outliers     : {} / {} ({:.1}%)",
        outliers, records.len(), 100.0 * outliers as f64 / records.len() as f64);
    println!("  mean distance: {:.3}", mean_d);
    println!("  max  distance: {:.3} at ts={}", max_d, max_ts);
}

fn transitions_cmd(args: &[String]) {
    // Detect regime changes in firing count across consecutive vitals samples.
    // `threshold` = minimum |Δfiring| to count as a transition.
    let threshold: i64 = args.iter().position(|a| a == "--threshold" || a == "-t")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);

    let log = home_dir().join(".airgenome").join("vitals.jsonl");
    let body = match std::fs::read_to_string(&log) {
        Ok(s) => s,
        Err(e) => { eprintln!("cannot read {}: {}", log.display(), e); std::process::exit(1); }
    };
    let records = airgenome::parse_log(&body);
    if records.len() < 2 { println!("need at least 2 records"); return; }

    let mut rising = 0usize;
    let mut falling = 0usize;
    let mut dwell_times: Vec<u64> = Vec::new(); // seconds spent at a stable level
    let mut last_transition_ts = records[0].ts;

    println!("=== airgenome — regime transitions ({} samples, threshold ±{}) ===",
        records.len(), threshold);
    println!();
    println!("  ts          firing     delta  direction");
    println!("  ────────────────────────────────────────");
    let mut shown = 0;
    for i in 1..records.len() {
        let a = records[i-1].firing as i64;
        let b = records[i].firing as i64;
        let d = b - a;
        if d.abs() >= threshold {
            if d > 0 { rising += 1; } else { falling += 1; }
            dwell_times.push(records[i].ts.saturating_sub(last_transition_ts));
            last_transition_ts = records[i].ts;
            let dir = if d > 0 { red("↑") } else { green("↓") };
            if shown < 15 {
                println!("  {}  {:>3} → {:<3}  {:>+3}     {}",
                    records[i].ts, a, b, d, dir);
                shown += 1;
            }
        }
    }
    if shown == 0 { println!("  (no transitions above threshold)"); }

    println!();
    println!("Summary:");
    println!("  rising  edges: {}", rising);
    println!("  falling edges: {}", falling);
    println!("  total  edges: {}", rising + falling);
    if !dwell_times.is_empty() {
        let mean: f64 = dwell_times.iter().sum::<u64>() as f64 / dwell_times.len() as f64;
        let mx = *dwell_times.iter().max().unwrap_or(&0);
        let mn = *dwell_times.iter().min().unwrap_or(&0);
        println!("  dwell (sec): mean={:.0} min={} max={}", mean, mn, mx);
    }
}

fn idle_capacity_cmd() {
    let log = home_dir().join(".airgenome").join("vitals.jsonl");
    let body = match std::fs::read_to_string(&log) {
        Ok(s) => s,
        Err(e) => { eprintln!("cannot read {}: {}", log.display(), e); std::process::exit(1); }
    };
    let records = airgenome::parse_log(&body);
    if records.is_empty() { println!("no records yet"); return; }

    // Per-axis stats: mean, min, max, stddev.
    let n = records.len() as f64;
    let mut sum = [0.0f64; 6];
    let mut sum_sq = [0.0f64; 6];
    let mut mn = [f64::INFINITY; 6];
    let mut mx = [f64::NEG_INFINITY; 6];

    for r in &records {
        let v = [r.cpu, r.ram, r.gpu, r.npu, r.power, r.io];
        for i in 0..6 {
            sum[i] += v[i];
            sum_sq[i] += v[i] * v[i];
            if v[i] < mn[i] { mn[i] = v[i]; }
            if v[i] > mx[i] { mx[i] = v[i]; }
        }
    }
    let mean: [f64; 6] = core::array::from_fn(|i| sum[i] / n);
    let var: [f64; 6] = core::array::from_fn(|i| sum_sq[i] / n - mean[i] * mean[i]);
    let stddev: [f64; 6] = core::array::from_fn(|i| var[i].max(0.0).sqrt());
    let names = ["cpu", "ram", "gpu", "npu", "power", "io"];

    println!("=== airgenome — idle capacity ({} samples) ===", records.len());
    println!();
    println!("  axis    mean     min    max   stddev   range");
    println!("  ─────────────────────────────────────────────");
    for i in 0..6 {
        let range = mx[i] - mn[i];
        println!("  {:<6}  {:>5.2}  {:>5.2}  {:>5.2}  {:>6.3}  {:>5.2}",
            names[i], mean[i], mn[i], mx[i], stddev[i], range);
    }
    println!();

    // Idle detection heuristic:
    // axis is "idle" if stddev is very small AND mean is either at 0 (unused)
    // or at the hardware max (GPU/NPU pinned to 8 cores but not truly busy).
    println!("Idle candidates:");
    let mut any = false;
    for i in 0..6 {
        let axis = names[i];
        // For gpu/npu, stddev < 0.01 with value > 0 suggests "always at max" = hw hint, not real utilization
        if (axis == "gpu" || axis == "npu") && stddev[i] < 0.01 && mean[i] > 0.0 {
            println!("  {}: stddev={:.3} (pinned at {:.1}) — likely hardware hint, real utilization unknown",
                axis, stddev[i], mean[i]);
            println!("    → potential offload target if workload is CPU-bound");
            any = true;
        }
        // For other axes: low mean AND low stddev = actually idle
        if axis != "gpu" && axis != "npu" && axis != "power" {
            if mean[i] < 0.1 && stddev[i] < 0.1 {
                println!("  {}: mean={:.3} stddev={:.3} — consistently idle",
                    axis, mean[i], stddev[i]);
                any = true;
            }
        }
    }
    if !any {
        println!("  (no axes meet idle thresholds)");
    }
}

fn insights_cmd(_args: &[String]) {
    let log = home_dir().join(".airgenome").join("vitals.jsonl");
    let body = match std::fs::read_to_string(&log) {
        Ok(s) => s,
        Err(e) => { eprintln!("cannot read {}: {}", log.display(), e); std::process::exit(1); }
    };
    let records = airgenome::parse_log(&body);
    if records.is_empty() {
        println!("no records yet — run daemon first.");
        return;
    }

    // Per-pair firing history via replay through a fresh engine.
    let mut per_pair = [0usize; 15];
    let mut engine = airgenome::PolicyEngine::with_defaults(12);
    let mut ram_hist: Vec<f64> = Vec::with_capacity(records.len());
    let mut hourly = [0usize; 24];        // firing count per hour
    let mut hourly_n = [0usize; 24];
    for r in &records {
        let mut axes = [0.0; 6];
        axes[Axis::Cpu.index()] = r.cpu;
        axes[Axis::Ram.index()] = r.ram;
        axes[Axis::Gpu.index()] = r.gpu;
        axes[Axis::Npu.index()] = r.npu;
        axes[Axis::Power.index()] = r.power;
        axes[Axis::Io.index()] = r.io;
        let v = airgenome::Vitals { ts: r.ts, axes };
        for p in engine.tick(v) { per_pair[p.pair] += 1; }
        ram_hist.push(r.ram);
        let h = ((r.ts / 3600) % 24) as usize;
        hourly[h] += r.firing;
        hourly_n[h] += 1;
    }

    println!("=== airgenome — insights ({} samples, {:.1}h span) ===",
        records.len(),
        (records.last().unwrap().ts - records.first().unwrap().ts) as f64 / 3600.0);
    println!();

    // 1. Top firing pairs
    let mut ranked: Vec<(usize, usize)> = per_pair.iter().copied().enumerate().collect();
    ranked.sort_by(|a,b| b.1.cmp(&a.1));
    println!("Top firing pairs (all time):");
    for (k, n) in ranked.iter().take(5) {
        if *n == 0 { continue; }
        let (a, b) = PAIRS[*k];
        println!("  [{:>2}] {:<6}×{:<6}  {:>5} fires", k, a.name(), b.name(), n);
    }
    println!();

    // 2. RAM trend (simple: first-third mean vs last-third mean)
    let n = ram_hist.len();
    if n >= 3 {
        let t1: f64 = ram_hist[..n/3].iter().sum::<f64>() / (n/3) as f64;
        let t3: f64 = ram_hist[n-n/3..].iter().sum::<f64>() / (n/3) as f64;
        let delta = t3 - t1;
        let trend = if delta > 0.02 { "rising" }
                    else if delta < -0.02 { "falling" }
                    else { "stable" };
        println!("RAM trend: {} ({:.3} → {:.3}, Δ{:+.3})", trend, t1, t3, delta);
    }
    println!();

    // 3. Hourly firing mean (UTC hour bucket — approximate local)
    let mut max_h = 0;
    let mut min_h = 24;
    let mut max_v = 0usize;
    let mut min_v = usize::MAX;
    println!("Hour-of-day firing (UTC):");
    for h in 0..24 {
        if hourly_n[h] == 0 { continue; }
        let mean = hourly[h] / hourly_n[h];
        if mean > max_v { max_v = mean; max_h = h; }
        if mean < min_v { min_v = mean; min_h = h; }
    }
    if max_v > 0 {
        println!("  peak hour : {:02}:00 UTC  ({} firing avg)", max_h, max_v);
        println!("  quiet hour: {:02}:00 UTC  ({} firing avg)", min_h, min_v);
    }
    println!();

    // 4. Profile recommendation (based on top firing pairs)
    // If battery-related pairs dominate → battery-save
    // If gpu/npu dominate → ml-inference
    // Default → balanced
    let battery_sum: usize = [3,7,10,12,14].iter().map(|&k| per_pair[k]).sum();
    let ml_sum: usize = [1,2,6,9,13].iter().map(|&k| per_pair[k]).sum();
    let dev_sum: usize = [0,4,8].iter().map(|&k| per_pair[k]).sum();
    let total: usize = per_pair.iter().sum();
    if total > 0 {
        println!("Profile fit:");
        println!("  battery-save : {:>5.1}%", 100.0 * battery_sum as f64 / total as f64);
        println!("  ml-inference : {:>5.1}%", 100.0 * ml_sum as f64 / total as f64);
        println!("  dev-work     : {:>5.1}%", 100.0 * dev_sum as f64 / total as f64);
        let recommended = if battery_sum > ml_sum && battery_sum > dev_sum { "battery-save" }
                          else if ml_sum > dev_sum { "ml-inference" }
                          else if dev_sum > 0 { "dev-work" }
                          else { "balanced" };
        println!("  recommended  : {}", green(recommended));
    }
}

fn coverage_cmd() {
    println!("=== airgenome — coverage matrix ===");
    println!();
    println!("  pair              T1  T2sh  T2sys  rule");
    println!("  ──────────────────────────────────────────────");
    let mut t1 = 0;
    let mut t2sh = 0;
    let mut t2sys = 0;
    for k in 0..PAIR_COUNT {
        let (a, b) = PAIRS[k];
        let tier1 = airgenome::plan_for_pair(k).is_some();
        let sudo_cmds = airgenome::commands_for(k)
            .map(|cs| cs.iter().any(|c| c.needs_sudo))
            .unwrap_or(false);
        let tier2_sysctl = airgenome::privileged::plan_tier2_for_pair(k).is_some();
        if tier1 { t1 += 1; }
        if sudo_cmds { t2sh += 1; }
        if tier2_sysctl { t2sys += 1; }
        let t1_m = if tier1 { green("●") } else { dim("·") };
        let t2sh_m = if sudo_cmds { yellow("●") } else { dim("·") };
        let t2sys_m = if tier2_sysctl { red("●") } else { dim("·") };
        println!("  [{:>2}] {:<6}×{:<6}  {}    {}    {}    {}",
            k, a.name(), b.name(), t1_m, t2sh_m, t2sys_m, green("●"));
    }
    println!();
    println!("  T1   (user-space)        : {:>2}/{} ({:.0}%)",
        t1, PAIR_COUNT, 100.0 * t1 as f64 / PAIR_COUNT as f64);
    println!("  T2sh (sudo shell cmds)   : {:>2}/{} ({:.0}%)",
        t2sh, PAIR_COUNT, 100.0 * t2sh as f64 / PAIR_COUNT as f64);
    println!("  T2sys(helper sysctl auto): {:>2}/{} ({:.0}%)",
        t2sys, PAIR_COUNT, 100.0 * t2sys as f64 / PAIR_COUNT as f64);
    println!("  Rules                    : {:>2}/{} (100%)", PAIR_COUNT, PAIR_COUNT);
}

fn sysctl_cmd(args: &[String]) {
    use airgenome::client::{dial, req_sysctl_get, HelperResponse, DEFAULT_SOCKET_PATH};
    let Some(key) = args.get(2) else {
        eprintln!("usage: airgenome sysctl <key>");
        eprintln!("whitelisted: {:?}", airgenome::privileged::SYSCTL_WHITELIST);
        std::process::exit(2);
    };
    let socket = std::env::var("AIRGENOME_HELPER_SOCKET")
        .unwrap_or_else(|_| DEFAULT_SOCKET_PATH.to_string());
    match dial(&socket, &req_sysctl_get(key)) {
        Ok(HelperResponse::Ok { detail }) => println!("{}", detail),
        Ok(HelperResponse::Refused { reason }) => { println!("refused: {}", reason); std::process::exit(1); }
        Ok(HelperResponse::Error { message }) => { println!("error: {}", message); std::process::exit(1); }
        Err(e) => { eprintln!("dial failed: {:?}", e); std::process::exit(1); }
    }
}

fn reap_cmd(args: &[String]) {
    use airgenome::client::{dial, req_purge, HelperResponse, DEFAULT_SOCKET_PATH};
    let yes = args.iter().any(|a| a == "--yes" || a == "-y");
    let measure = args.iter().any(|a| a == "--measure" || a == "-m");

    println!("=== airgenome reap — RAM-focused combo ===");
    println!();
    let before = airgenome::sample();
    println!("  before: ram={:.3} firing={}/15",
        before.get(Axis::Ram), airgenome::firing(&before).len());

    if !yes {
        println!();
        println!("  plan:");
        println!("    1. Tier 1: pkill -TERM -f 'Google Chrome Helper (Renderer)'");
        println!("    2. Tier 1: pkill -TERM -f 'Slack Helper'");
        println!("    3. Tier 2: helper purge (if socket present)");
        println!();
        println!("{}", dim("dry-run. pass --yes to execute."));
        return;
    }

    // 1. Tier 1 kills
    for pat in &["Google Chrome Helper (Renderer)", "Slack Helper"] {
        let a = airgenome::UserAction::KillProcess { pattern: pat.to_string() };
        match airgenome::execute(&a) {
            Ok(r) => println!("  tier1 kill '{}' → exit={:?}", pat, r.exit_code),
            Err(e) => println!("  tier1 kill '{}' → abort {:?}", pat, e),
        }
    }

    // 2. Tier 2 purge
    let socket = std::env::var("AIRGENOME_HELPER_SOCKET")
        .unwrap_or_else(|_| DEFAULT_SOCKET_PATH.to_string());
    match dial(&socket, &req_purge()) {
        Ok(HelperResponse::Ok { detail }) => println!("  tier2 {} {}", green("purge"), detail),
        Ok(HelperResponse::Refused { reason }) => println!("  tier2 {} {}", yellow("purge refused"), reason),
        Ok(HelperResponse::Error { message }) => println!("  tier2 {} {}", red("purge error"), message),
        Err(_) => println!("  tier2 {} (helper not installed, skipped)", dim("purge")),
    }

    if measure {
        std::thread::sleep(std::time::Duration::from_secs(3));
        let after = airgenome::sample();
        let dram = after.get(Axis::Ram) - before.get(Axis::Ram);
        let before_firing = airgenome::firing(&before).len();
        let after_firing = airgenome::firing(&after).len();
        println!();
        println!("{}", dim("--- delta ---"));
        println!("  ram      {:.3} → {:.3}  ({:+.3})",
            before.get(Axis::Ram), after.get(Axis::Ram), dram);
        println!("  firing   {} → {}  ({:+})",
            before_firing, after_firing, (after_firing as i64) - (before_firing as i64));
        let verdict = if dram < -0.02 || after_firing < before_firing { green("improved") }
                      else if dram > 0.02 || after_firing > before_firing { red("worse") }
                      else { dim("unchanged") };
        println!("  verdict: {}", verdict);
    }
}

fn tune_cmd(args: &[String]) {
    use airgenome::client::{dial, req_sysctl_get, req_sysctl_set, HelperResponse, DEFAULT_SOCKET_PATH};
    let (Some(key), Some(value)) = (args.get(2), args.get(3)) else {
        eprintln!("usage: airgenome tune <key> <value> [--measure]");
        eprintln!("whitelisted keys:");
        for k in airgenome::privileged::SYSCTL_WHITELIST { eprintln!("  {}", k); }
        std::process::exit(2);
    };
    let measure = args.iter().any(|a| a == "--measure" || a == "-m");
    let socket = std::env::var("AIRGENOME_HELPER_SOCKET")
        .unwrap_or_else(|_| DEFAULT_SOCKET_PATH.to_string());

    // Read current value first.
    if let Ok(HelperResponse::Ok { detail }) = dial(&socket, &req_sysctl_get(key)) {
        println!("{} {}", dim("before:"), detail);
    }

    let before_firing = if measure {
        Some(airgenome::firing(&airgenome::sample()).len())
    } else { None };

    match dial(&socket, &req_sysctl_set(key, value)) {
        Ok(HelperResponse::Ok { detail }) => {
            println!("{} {}", green("tuned"), detail);
            if let Some(bf) = before_firing {
                std::thread::sleep(std::time::Duration::from_secs(2));
                let af = airgenome::firing(&airgenome::sample()).len();
                println!("{}", dim("--- delta ---"));
                println!("  firing {} → {}  ({:+})", bf, af, (af as i64) - (bf as i64));
            }
        }
        Ok(HelperResponse::Refused { reason }) => {
            println!("{} {}", yellow("refused"), reason);
            std::process::exit(1);
        }
        Ok(HelperResponse::Error { message }) => {
            println!("{} {}", red("error"), message);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("dial failed: {:?}", e);
            eprintln!("install helper first: sudo bash scripts/install-helper.sh install");
            std::process::exit(1);
        }
    }
}

fn purge_cmd(args: &[String]) {
    use airgenome::client::{dial, req_purge, HelperResponse, DEFAULT_SOCKET_PATH};
    let measure = args.iter().any(|a| a == "--measure" || a == "-m");
    let socket = std::env::var("AIRGENOME_HELPER_SOCKET")
        .unwrap_or_else(|_| DEFAULT_SOCKET_PATH.to_string());

    let before = if measure { Some(airgenome::sample()) } else { None };

    match dial(&socket, &req_purge()) {
        Ok(HelperResponse::Ok { detail }) => {
            println!("{} {}", green("ok"), detail);
            if let Some(before) = before {
                std::thread::sleep(std::time::Duration::from_secs(2));
                let after = airgenome::sample();
                let dram = after.get(Axis::Ram) - before.get(Axis::Ram);
                println!();
                println!("{}", dim("--- delta ---"));
                println!("  ram  {:.3} → {:.3}  ({:+.3})",
                    before.get(Axis::Ram), after.get(Axis::Ram), dram);
                let verdict = if dram < -0.02 { green("improved") }
                              else if dram > 0.02 { red("worse") }
                              else { dim("unchanged") };
                println!("  verdict: {}", verdict);
            }
        }
        Ok(HelperResponse::Refused { reason }) => {
            println!("{} {}", yellow("refused"), reason);
            std::process::exit(1);
        }
        Ok(HelperResponse::Error { message }) => {
            println!("{} {}", red("error"), message);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("dial failed: {:?}", e);
            eprintln!("install the helper first: sudo bash scripts/install-helper.sh install");
            std::process::exit(1);
        }
    }
}

fn helper_cmd(args: &[String]) {
    use airgenome::client::{dial, req_ping, req_sysctl_get, req_sysctl_set, req_purge, HelperResponse, DEFAULT_SOCKET_PATH};

    let op = args.get(2).map(|s| s.as_str()).unwrap_or("ping");
    let socket = std::env::var("AIRGENOME_HELPER_SOCKET")
        .unwrap_or_else(|_| DEFAULT_SOCKET_PATH.to_string());

    let request = match op {
        "ping" => req_ping(),
        "get" | "sysctl-get" => {
            let Some(key) = args.get(3) else {
                eprintln!("usage: airgenome helper get <key>");
                std::process::exit(2);
            };
            req_sysctl_get(key)
        }
        "set" | "sysctl-set" => {
            let (Some(key), Some(value)) = (args.get(3), args.get(4)) else {
                eprintln!("usage: airgenome helper set <key> <value>");
                std::process::exit(2);
            };
            req_sysctl_set(key, value)
        }
        "purge" => req_purge(),
        _ => {
            eprintln!("usage: airgenome helper <ping|get|set|purge> [args]");
            eprintln!("  env AIRGENOME_HELPER_SOCKET overrides path (default {})", DEFAULT_SOCKET_PATH);
            std::process::exit(2);
        }
    };

    match dial(&socket, &request) {
        Ok(HelperResponse::Ok { detail }) => {
            println!("{} {}", green("ok"), detail);
        }
        Ok(HelperResponse::Refused { reason }) => {
            println!("{} {}", yellow("refused"), reason);
            std::process::exit(1);
        }
        Ok(HelperResponse::Error { message }) => {
            println!("{} {}", red("error"), message);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("dial failed: {:?}", e);
            eprintln!("hint: is airgenome-helper running? socket={}", socket);
            eprintln!("install: sudo bash scripts/install-helper.sh install");
            std::process::exit(1);
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

    // 7. Tier 2 helper (optional).
    let sock_path = std::env::var("AIRGENOME_HELPER_SOCKET")
        .unwrap_or_else(|_| airgenome::client::DEFAULT_SOCKET_PATH.to_string());
    if std::path::Path::new(&sock_path).exists() {
        match airgenome::client::dial(&sock_path, &airgenome::client::req_ping()) {
            Ok(airgenome::client::HelperResponse::Ok { .. }) => {
                ok("helper (Tier 2)", &sock_path);
                pass += 1;
            }
            Ok(other) => {
                wrn("helper (Tier 2)", &format!("socket up but peer not authenticated: {:?}", other));
                warn += 1;
            }
            Err(_) => {
                wrn("helper (Tier 2)", "socket exists but dial failed");
                warn += 1;
            }
        }
    } else {
        wrn("helper (Tier 2)", "not installed (optional — run install-helper.sh)");
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
    println!("  simulate <scenario> run full pipeline against synthetic vitals");
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
    println!("  helper <ping|get|set|purge> [args]        talk to privileged helper (Tier 2)");
    println!("  purge [--measure]                         request memory purge via helper");
    println!("  tune <key> <value> [--measure]            sysctl tune via helper (whitelisted)");
    println!("  sysctl <key>                              read a whitelisted sysctl via helper");
    println!("  reap [--yes] [--measure]                  RAM-focused combo: kill Chrome/Slack + purge");
    println!("  coverage                                  15-pair × tier coverage matrix");
    println!("  insights                                  extract patterns from vitals.jsonl history");
    println!("  idle-capacity                             per-axis stats + idle axis detection");
    println!("  transitions [-t N]                        regime changes in firing count (|Δ|≥N)");
    println!("  anomalies [-t D]                          samples where min fingerprint distance > D");
    println!("  processes                                 categorize current procs by app family (RSS/CPU)");
    println!("  signature [cat] [--append|--json]         6-axis signature per category");
    println!("  signature-history [cat]                   aggregate signatures.jsonl history");
    println!("  fingerprints                              list built-in + custom fingerprints");
    println!("  fingerprint-save <name>                   save current vitals as custom fingerprint");
    println!("  match [--append|--json]                   match current vitals → nearest fingerprint");
    println!("  match-distribution                        workload distribution from matches.jsonl");
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
