//! airgenome CLI — probe, status, diagnostics, profile management.

use airgenome::{self, Axis, AXIS_COUNT, PAIR_COUNT, PAIRS, GENOME_BYTES, RULES};

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
    println!("  help                print this help");
}
