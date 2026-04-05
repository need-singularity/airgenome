//! airgenome CLI — probe, status, genome inspection.

use airgenome::{self, Axis, AXIS_COUNT, PAIR_COUNT, PAIRS, GENOME_BYTES};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let sub = args.get(1).map(|s| s.as_str()).unwrap_or("status");

    match sub {
        "status" | "st" => status(),
        "probe" | "pr" => probe(),
        "pairs" => list_pairs(),
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

fn print_help() {
    println!("airgenome — 6-axis Mac Air resource hexagon\n");
    println!("USAGE:");
    println!("  airgenome [SUBCOMMAND]\n");
    println!("SUBCOMMANDS:");
    println!("  status   show hexagon + one vitals sample (default)");
    println!("  probe    emit a single JSON vitals sample");
    println!("  pairs    list the 15 canonical pair gates");
    println!("  help     print this help");
}
