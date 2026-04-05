//! airgenome CLI — probe, status, diagnostics, profile management.

use airgenome::{self, Axis, Genome, AXIS_COUNT, PAIR_COUNT, PAIRS, GENOME_BYTES, RULES};
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let sub = args.get(1).map(|s| s.as_str()).unwrap_or("status");

    match sub {
        "status" | "st" => status(),
        "probe" | "pr" => probe(),
        "pairs" => list_pairs(),
        "rules" => list_rules(),
        "diag" => diag(),
        "profile" => profile_cmd(&args),
        "diff" => diff_cmd(&args),
        "daemon" => daemon_cmd(&args),
        "trace" => trace_cmd(&args),
        "help" | "-h" | "--help" => print_help(),
        other => {
            eprintln!("unknown sub-command: '{}'", other);
            print_help();
            std::process::exit(2);
        }
    }
}

fn status() {
    let v = airgenome::sample();
    println!("=== airgenome — Mac Air Implant Status ===");
    println!("  Hexagon: {} axes × {} pairs | genome = {} bytes",
        AXIS_COUNT, PAIR_COUNT, GENOME_BYTES);
    println!();
    println!("  Axes (vitals sample @ ts={}):", v.ts);
    for axis in Axis::ALL {
        println!("    {:<6} {:>10.4}", axis.name(), v.get(axis));
    }
    println!();
    let f = airgenome::firing(&v);
    println!("  Rules firing: {} / {}", f.len(), PAIR_COUNT);
    println!("  Meta fixed point: 1/3 ≈ {:.6}  (work = 2/3 ≈ {:.6})",
        airgenome::META_FP, airgenome::WORK_FP);
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

fn diag() {
    let v = airgenome::sample();
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
            airgenome::Severity::Critical => { critical += 1; "CRITICAL" }
            airgenome::Severity::Warn => { warn += 1; "warn    " }
            airgenome::Severity::Ok => "ok      ",
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
    println!("Built-in profiles ({} total):", airgenome::PROFILES.len());
    for p in airgenome::PROFILES.iter() {
        println!("  {:<14} {:>2} pairs  — {}",
            p.name, p.active_count(), p.description);
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
        eprintln!("usage: airgenome profile apply <name>");
        std::process::exit(2);
    };
    let Some(p) = airgenome::by_name(name) else {
        eprintln!("unknown profile: '{}'", name);
        std::process::exit(2);
    };

    let dir = home_dir().join(".airgenome");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("cannot create {}: {}", dir.display(), e);
        std::process::exit(1);
    }
    let path = dir.join("active.genome");
    let bytes = p.genome().to_bytes();
    if let Err(e) = std::fs::write(&path, bytes) {
        eprintln!("cannot write {}: {}", path.display(), e);
        std::process::exit(1);
    }
    println!("applied profile '{}' → {}", p.name, path.display());
    println!("  {} pairs engaged (60 bytes written)", p.active_count());
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
    // parse --interval N (seconds), --output PATH
    let mut interval_s: u64 = 30;
    let mut output: Option<std::path::PathBuf> = None;
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

    eprintln!("airgenome daemon — interval {}s, log {}", interval_s, out_path.display());
    eprintln!("Ctrl+C to stop.");

    loop {
        let v = airgenome::sample();
        let firing = airgenome::firing(&v).len();
        let line = format!(
            "{{\"ts\":{},\"cpu\":{},\"ram\":{},\"gpu\":{},\"npu\":{},\"power\":{},\"io\":{},\"firing\":{}}}",
            v.ts, v.get(Axis::Cpu), v.get(Axis::Ram),
            v.get(Axis::Gpu), v.get(Axis::Npu),
            v.get(Axis::Power), v.get(Axis::Io), firing
        );
        if writeln!(file, "{}", line).is_err() {
            eprintln!("write failed; exiting");
            std::process::exit(1);
        }
        let _ = file.flush();
        eprintln!("[{}] firing={}/{}  cpu={:.2} ram={:.2} io={:.2}",
            v.ts, firing, PAIR_COUNT,
            v.get(Axis::Cpu), v.get(Axis::Ram), v.get(Axis::Io));
        std::thread::sleep(std::time::Duration::from_secs(interval_s));
    }
}

fn trace_cmd(args: &[String]) {
    // parse --input PATH, --tail N
    let mut input: Option<std::path::PathBuf> = None;
    let mut tail: Option<usize> = None;
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
    println!("  pairs               list the 15 canonical pair gates");
    println!("  rules               list the 15 rules with mesh neighbors");
    println!("  diag                fire rules on current vitals + dry-run proposals");
    println!("  profile list        list built-in profiles");
    println!("  profile show NAME   show engaged pairs + genome hex");
    println!("  profile apply NAME  write profile to ~/.airgenome/active.genome");
    println!("  profile active      show the currently applied profile");
    println!("  diff A B            show per-pair genome differences between profiles");
    println!("  daemon [-i SEC]     periodic vitals loop → ~/.airgenome/vitals.jsonl");
    println!("  trace [--tail N]    summarize ~/.airgenome/vitals.jsonl");
    println!("  help                print this help");
}
