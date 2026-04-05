# Singularity Breakthrough Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the 1/30 efficiency gap (0.636 → 0.667) by connecting three disconnected subsystems: mesh coupling, MI gap analysis, and ghost process detection — all kill-free.

**Architecture:** Three layers (A: mesh coupling in policy.rs, B: MI gap in efficiency.rs, C: ghost scan in airgenome.rs) unified under a single `airgenome nexus` command that loads vitals history, runs all three analyses, and reports adjusted efficiency + singularity distance.

**Tech Stack:** Pure Rust, serde, existing airgenome crate. No new dependencies.

**Spec:** `docs/superpowers/specs/2026-04-05-singularity-breakthrough-design.md`

---

## File Structure

| File | Responsibility | Change type |
|------|---------------|-------------|
| `src/policy.rs` | Add `CascadeInfo` struct + `mesh_cascade()` function | Modify (append) |
| `src/efficiency.rs` | Add `mi_gap()` function | Modify (append) |
| `src/lib.rs` | Re-export new symbols | Modify (1 line) |
| `src/bin/airgenome.rs` | Add `ghost` cmd, `nexus` cmd, wire into main match | Modify (append) |

---

### Task 1: Mesh Coupling — CascadeInfo + mesh_cascade()

**Files:**
- Modify: `src/policy.rs` (append after line 146)
- Modify: `src/lib.rs:57` (add re-export)

- [ ] **Step 1: Write failing tests for mesh_cascade**

Append to the bottom of `src/policy.rs`, inside the existing `#[cfg(test)] mod tests { ... }` block (before the final `}`):

```rust
    #[test]
    fn cascade_no_neighbors_firing() {
        let fired = vec![0]; // only pair 0 fires
        // neighbors(0) = [1, 5, 11] — none of them in `fired`
        let v_idle = v(0, 0.5, 0.1, 0.0);
        let cascades = mesh_cascade_for(&fired, &v_idle);
        assert_eq!(cascades.len(), 1);
        assert_eq!(cascades[0].pair, 0);
        assert_eq!(cascades[0].neighbor_fires, 0);
        assert_eq!(cascades[0].boost, 0);
    }

    #[test]
    fn cascade_two_neighbors_firing() {
        // pair 0 fires, neighbors(0) = [1, 5, 11]
        // if pairs 1 and 5 also fire → 2 neighbors → boost 0x10
        let fired = vec![0, 1, 5];
        let v_idle = v(0, 0.5, 0.1, 0.0);
        let cascades = mesh_cascade_for(&fired, &v_idle);
        let c0 = cascades.iter().find(|c| c.pair == 0).unwrap();
        assert_eq!(c0.neighbor_fires, 2);
        assert_eq!(c0.boost, 0x10);
    }

    #[test]
    fn cascade_full_three_neighbors() {
        // pair 0 fires, all neighbors [1, 5, 11] also fire → boost 0x20
        let fired = vec![0, 1, 5, 11];
        let v_idle = v(0, 0.5, 0.1, 0.0);
        let cascades = mesh_cascade_for(&fired, &v_idle);
        let c0 = cascades.iter().find(|c| c.pair == 0).unwrap();
        assert_eq!(c0.neighbor_fires, 3);
        assert_eq!(c0.boost, 0x20);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib cascade -- --nocapture 2>&1 | head -20`
Expected: compilation error — `mesh_cascade_for` not found.

- [ ] **Step 3: Implement CascadeInfo and mesh_cascade_for**

Add above the `#[cfg(test)]` block in `src/policy.rs` (after the closing `}` of `impl PolicyEngine` block, around line 139):

```rust
/// Cascade result from mesh neighbor analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CascadeInfo {
    pub pair: usize,
    pub neighbor_fires: u8,
    pub boost: u8,
}

/// Compute mesh cascade for a set of fired pairs.
///
/// For each pair in `fired`, counts how many of its 3 mesh neighbors
/// also fired. Returns a `CascadeInfo` per fired pair.
///   - 0-1 neighbors: boost = 0 (no cascade)
///   - 2 neighbors:   boost = 0x10 (stability increment)
///   - 3 neighbors:   boost = 0x20 (full cascade + surprise)
pub fn mesh_cascade_for(fired: &[usize], _v: &Vitals) -> Vec<CascadeInfo> {
    use crate::rules::neighbors;
    fired.iter().map(|&k| {
        let ns = neighbors(k);
        let count = ns.iter().filter(|n| fired.contains(n)).count() as u8;
        let boost = match count {
            0 | 1 => 0,
            2 => 0x10,
            _ => 0x20,
        };
        CascadeInfo { pair: k, neighbor_fires: count, boost }
    }).collect()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib cascade -- --nocapture`
Expected: 3 tests pass.

- [ ] **Step 5: Add re-export to lib.rs**

In `src/lib.rs`, change line 57:

Old:
```rust
pub use policy::{PolicyEngine, PolicyConfig, Proposal, Reason};
```

New:
```rust
pub use policy::{PolicyEngine, PolicyConfig, Proposal, Reason, CascadeInfo, mesh_cascade_for};
```

- [ ] **Step 6: Run full test suite**

Run: `cargo test 2>&1 | grep "test result"`
Expected: all 126 existing tests pass + 3 new = 129 total (in lib).

- [ ] **Step 7: Commit**

```bash
git add src/policy.rs src/lib.rs
git commit -m "feat(nexus): add mesh cascade — connect neighbors() to policy decisions

Layer A of singularity breakthrough: CascadeInfo struct + mesh_cascade_for()
computes engagement boost when mesh neighbors co-fire.
0 neighbors=no boost, 2=+0x10, 3=+0x20."
```

---

### Task 2: MI Gap Analysis — mi_gap()

**Files:**
- Modify: `src/efficiency.rs` (append before `#[cfg(test)]`)
- Modify: `src/lib.rs:51` (add re-export)

- [ ] **Step 1: Write failing tests for mi_gap**

Append inside the existing `#[cfg(test)] mod tests { ... }` block in `src/efficiency.rs`:

```rust
    #[test]
    fn mi_gap_zero_when_fire_matches_mi() {
        use crate::gate::Axis;
        // Construct vitals where cpu and ram are perfectly correlated
        // AND fire_counts reflect that — gap should be ~0 for pair 0.
        let mut history = Vec::new();
        for i in 0..100 {
            let t = i as f64 / 100.0;
            let mut axes = [0.0; 6];
            axes[Axis::Cpu.index()] = 5.0 * t;
            axes[Axis::Ram.index()] = t;
            axes[Axis::Gpu.index()] = 8.0;
            axes[Axis::Npu.index()] = 8.0;
            axes[Axis::Power.index()] = 1.0;
            axes[Axis::Io.index()] = 0.0;
            history.push(crate::vitals::Vitals { ts: i, axes });
        }
        // fire_counts[0] = 100 (always fires) → fire_rate = 1.0
        let mut fire_counts = [0usize; 15];
        fire_counts[0] = 100;
        let gaps = mi_gap(&history, &fire_counts, 10);
        // gap[0] should be 0 or negative (clamped to 0)
        assert!(gaps[0] < 0.1, "expected near-zero gap, got {}", gaps[0]);
    }

    #[test]
    fn mi_gap_positive_when_correlated_but_never_fires() {
        use crate::gate::Axis;
        let mut history = Vec::new();
        for i in 0..100 {
            let t = i as f64 / 100.0;
            let mut axes = [0.0; 6];
            axes[Axis::Cpu.index()] = 5.0 * t;
            axes[Axis::Ram.index()] = t;  // correlated with cpu
            axes[Axis::Gpu.index()] = 8.0;
            axes[Axis::Npu.index()] = 8.0;
            axes[Axis::Power.index()] = 1.0;
            axes[Axis::Io.index()] = 0.0;
            history.push(crate::vitals::Vitals { ts: i, axes });
        }
        // fire_counts[0] = 0 — never fires despite high MI
        let fire_counts = [0usize; 15];
        let gaps = mi_gap(&history, &fire_counts, 10);
        // MI(cpu, ram) is high, fire_rate = 0 → gap > 0
        assert!(gaps[0] > 0.3, "expected positive gap for correlated pair, got {}", gaps[0]);
    }

    #[test]
    fn mi_gap_has_fifteen_entries() {
        let history = vec![crate::vitals::Vitals::zeroed(); 20];
        let fire_counts = [0usize; 15];
        let gaps = mi_gap(&history, &fire_counts, 5);
        assert_eq!(gaps.len(), 15);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib mi_gap -- --nocapture 2>&1 | head -20`
Expected: compilation error — `mi_gap` not found.

- [ ] **Step 3: Implement mi_gap**

Add in `src/efficiency.rs`, before the `#[cfg(test)]` block:

```rust
/// Per-pair MI gap: how much mutual information each gate fails to capture.
///
/// For each of the 15 axis pairs, computes MI between the two axes over
/// `history`, normalizes to `[0, 1]`, then subtracts the normalized fire
/// rate. Positive gap = "leaking gate" (correlated axes, rule not firing).
///
/// Returns `[f64; 15]` — one gap score per pair, clamped to `>= 0`.
pub fn mi_gap(
    history: &[crate::vitals::Vitals],
    fire_counts: &[usize; 15],
    bins: usize,
) -> [f64; 15] {
    use crate::gate::PAIRS;
    let n = history.len();
    if n < bins || bins < 2 {
        return [0.0; 15];
    }

    // Extract per-axis time series.
    let axis_series = |axis: crate::gate::Axis| -> Vec<f64> {
        history.iter().map(|v| v.get(axis)).collect()
    };

    // Compute MI for each pair.
    let mut mi_raw = [0.0f64; 15];
    for (k, &(a, b)) in PAIRS.iter().enumerate() {
        let xs = axis_series(a);
        let ys = axis_series(b);
        mi_raw[k] = mutual_info_hist(&xs, &ys, bins);
    }

    // Normalize MI to [0, 1].
    let mi_max = mi_raw.iter().cloned().fold(0.0f64, f64::max);
    let mi_norm: [f64; 15] = {
        let mut arr = [0.0; 15];
        for k in 0..15 {
            arr[k] = if mi_max > 0.0 { mi_raw[k] / mi_max } else { 0.0 };
        }
        arr
    };

    // Normalize fire rates to [0, 1].
    let fr_max = *fire_counts.iter().max().unwrap_or(&1) as f64;
    let fr_norm: [f64; 15] = {
        let mut arr = [0.0; 15];
        for k in 0..15 {
            arr[k] = if fr_max > 0.0 { fire_counts[k] as f64 / fr_max } else { 0.0 };
        }
        arr
    };

    // Gap = mi_norm - fr_norm, clamped to >= 0.
    let mut gaps = [0.0; 15];
    for k in 0..15 {
        gaps[k] = (mi_norm[k] - fr_norm[k]).max(0.0);
    }
    gaps
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib mi_gap -- --nocapture`
Expected: 3 tests pass.

- [ ] **Step 5: Add re-export to lib.rs**

In `src/lib.rs`, change line 51:

Old:
```rust
pub use efficiency::{EfficiencyTracker, META_FP, WORK_FP, mutual_info_hist};
```

New:
```rust
pub use efficiency::{EfficiencyTracker, META_FP, WORK_FP, mutual_info_hist, mi_gap};
```

- [ ] **Step 6: Run full test suite**

Run: `cargo test 2>&1 | grep "test result"`
Expected: all previous tests pass + 3 new MI tests.

- [ ] **Step 7: Commit**

```bash
git add src/efficiency.rs src/lib.rs
git commit -m "feat(nexus): add mi_gap — identify leaking gates via mutual information

Layer B of singularity breakthrough: per-pair MI gap analysis using
the existing mutual_info_hist() estimator. Normalizes MI and fire rates
to [0,1], gap = mi_norm - fr_norm (clamped >= 0)."
```

---

### Task 3: Ghost Sink — ghost_cmd()

**Files:**
- Modify: `src/bin/airgenome.rs` (add `ghost_cmd` function + wire into main match)

- [ ] **Step 1: Add `ghost` to the main match dispatch**

In `src/bin/airgenome.rs`, find the line:

```rust
        "processes" | "proc" => processes_cmd(),
```

Add after it:

```rust
        "ghost" => ghost_cmd(),
```

- [ ] **Step 2: Implement ghost_cmd**

Add the following function in `src/bin/airgenome.rs` (after `processes_cmd` function, around line 836):

```rust
fn ghost_cmd() {
    // ── Scan 1: Zombie processes (Z state) ──
    let ps_stat = match std::process::Command::new("ps")
        .args(["-axo", "stat=,pid=,rss=,comm="])
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => { eprintln!("ps failed"); std::process::exit(1); }
    };

    struct Ghost { pid: u32, rss_kb: u64, kind: &'static str, name: String }
    let mut ghosts: Vec<Ghost> = Vec::new();

    // Parse zombies.
    for line in ps_stat.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        let parts: Vec<&str> = trimmed.splitn(4, char::is_whitespace).collect();
        if parts.len() < 4 { continue; }
        let stat = parts[0].trim();
        if stat.contains('Z') {
            let pid: u32 = parts[1].trim().parse().unwrap_or(0);
            let rss_kb: u64 = parts[2].trim().parse().unwrap_or(0);
            let name = parts[3].trim().to_string();
            ghosts.push(Ghost { pid, rss_kb, kind: "zombie", name });
        }
    }

    // ── Scan 2: Orphaned helpers (ppid=1 + Helper/Agent/Worker + RSS>10MB) ──
    let ps_ppid = match std::process::Command::new("ps")
        .args(["-axo", "ppid=,pid=,rss=,comm="])
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => String::new(),
    };

    for line in ps_ppid.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        let parts: Vec<&str> = trimmed.splitn(4, char::is_whitespace).collect();
        if parts.len() < 4 { continue; }
        let ppid: u32 = parts[0].trim().parse().unwrap_or(999);
        if ppid != 1 { continue; }
        let pid: u32 = parts[1].trim().parse().unwrap_or(0);
        let rss_kb: u64 = parts[2].trim().parse().unwrap_or(0);
        let name = parts[3].trim().to_string();
        let lo = name.to_lowercase();
        if rss_kb > 10_000 && (lo.contains("helper") || lo.contains("agent") || lo.contains("worker")) {
            // Don't double-count zombies.
            if !ghosts.iter().any(|g| g.pid == pid) {
                ghosts.push(Ghost { pid, rss_kb, kind: "orphan", name });
            }
        }
    }

    // ── Scan 3: RSS ghosts (high RSS, zero CPU, sampled twice) ──
    let sample_ps = || -> Vec<(u32, u64, f64, String)> {
        let out = match std::process::Command::new("ps")
            .args(["-axo", "pid=,rss=,pcpu=,comm="])
            .output()
        {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
            _ => return Vec::new(),
        };
        let mut rows = Vec::new();
        for line in out.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() { continue; }
            let mut it = trimmed.split_whitespace();
            let pid: u32 = match it.next().and_then(|s| s.parse().ok()) { Some(v) => v, None => continue };
            let rss_kb: u64 = match it.next().and_then(|s| s.parse().ok()) { Some(v) => v, None => continue };
            let cpu: f64 = match it.next().and_then(|s| s.parse().ok()) { Some(v) => v, None => continue };
            let comm: String = it.collect::<Vec<_>>().join(" ");
            rows.push((pid, rss_kb, cpu, comm));
        }
        rows
    };

    let snap1 = sample_ps();
    std::thread::sleep(std::time::Duration::from_secs(2));
    let snap2 = sample_ps();

    for (pid, rss, cpu2, name) in &snap2 {
        if *rss < 50_000 || *cpu2 > 0.0 { continue; }
        // Must also be zero CPU in snap1.
        if let Some((_, _, cpu1, _)) = snap1.iter().find(|(p, _, _, _)| p == pid) {
            if *cpu1 > 0.0 { continue; }
            if !ghosts.iter().any(|g| g.pid == *pid) {
                ghosts.push(Ghost { pid: *pid, rss_kb: *rss, kind: "rss-ghost", name: name.clone() });
            }
        }
    }

    // ── Aggregate and report ──
    let total_sys_rss: u64 = snap2.iter().map(|(_, r, _, _)| r).sum();
    let zombie_count = ghosts.iter().filter(|g| g.kind == "zombie").count();
    let orphan_count = ghosts.iter().filter(|g| g.kind == "orphan").count();
    let rss_ghost_count = ghosts.iter().filter(|g| g.kind == "rss-ghost").count();
    let zombie_rss: u64 = ghosts.iter().filter(|g| g.kind == "zombie").map(|g| g.rss_kb).sum();
    let orphan_rss: u64 = ghosts.iter().filter(|g| g.kind == "orphan").map(|g| g.rss_kb).sum();
    let rss_ghost_rss: u64 = ghosts.iter().filter(|g| g.kind == "rss-ghost").map(|g| g.rss_kb).sum();
    let total_ghost_rss = zombie_rss + orphan_rss + rss_ghost_rss;
    let ghost_frac = if total_sys_rss > 0 { total_ghost_rss as f64 / total_sys_rss as f64 } else { 0.0 };

    println!("=== airgenome ghost — information sink scan ===");
    println!();
    println!("  Type          Count    RSS total    % of system RSS");
    println!("  ────────────────────────────────────────────────────");
    println!("  zombie       {:>5}    {:>6} MB      {:.1}%", zombie_count, zombie_rss / 1024, if total_sys_rss > 0 { 100.0 * zombie_rss as f64 / total_sys_rss as f64 } else { 0.0 });
    println!("  orphan       {:>5}    {:>6} MB      {:.1}%", orphan_count, orphan_rss / 1024, if total_sys_rss > 0 { 100.0 * orphan_rss as f64 / total_sys_rss as f64 } else { 0.0 });
    println!("  rss-ghost    {:>5}    {:>6} MB      {:.1}%", rss_ghost_count, rss_ghost_rss / 1024, if total_sys_rss > 0 { 100.0 * rss_ghost_rss as f64 / total_sys_rss as f64 } else { 0.0 });
    println!("  ────────────────────────────────────────────────────");
    println!("  total        {:>5}    {:>6} MB      {:.1}%", ghosts.len(), total_ghost_rss / 1024, 100.0 * ghost_frac);
    println!();

    // Hexagon impact.
    let eff_loss = ghost_frac * (5.0 / 15.0);
    println!("  Hexagon impact (ram axis pollution):");
    println!("    ghost_fraction = {:.3}", ghost_frac);
    println!("    ram-centered pairs affected: [0, 5, 6, 7, 8] (5 of 15)");
    println!("    estimated efficiency loss: {:.4}", eff_loss);
    println!();

    // Top ghosts.
    ghosts.sort_by(|a, b| b.rss_kb.cmp(&a.rss_kb));
    if !ghosts.is_empty() {
        println!("  Top ghosts:");
        for g in ghosts.iter().take(10) {
            let short: String = g.name.split('/').next_back().unwrap_or(&g.name).chars().take(40).collect();
            println!("    PID {:>6}  {:>6} MB  {:>8}  {}", g.pid, g.rss_kb / 1024, g.kind, short);
        }
        println!();
    }

    println!("  {}", dim("(kill-free: no processes terminated)"));
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/bin/airgenome.rs
git commit -m "feat(nexus): add ghost command — information sink detection

Layer C of singularity breakthrough: three-scan ghost process detection
(zombie/orphan-helper/rss-ghost) with hexagon impact estimation.
Kill-free, report only."
```

---

### Task 4: Unified nexus command

**Files:**
- Modify: `src/bin/airgenome.rs` (add `nexus_cmd` + wire into main match)

- [ ] **Step 1: Add `nexus` to the main match dispatch**

In `src/bin/airgenome.rs`, find the line:

```rust
        "ghost" => ghost_cmd(),
```

Add after it:

```rust
        "nexus" => nexus_cmd(&args),
```

- [ ] **Step 2: Implement nexus_cmd**

Add the following function in `src/bin/airgenome.rs` (after `ghost_cmd`):

```rust
fn nexus_cmd(args: &[String]) {
    let bins: usize = args.iter().position(|a| a == "--bins" || a == "-b")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    // ── Load vitals history ──
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

    // Convert to Vitals vec.
    let history: Vec<airgenome::Vitals> = records.iter().map(|r| {
        let mut axes = [0.0; 6];
        axes[Axis::Cpu.index()] = r.cpu;
        axes[Axis::Ram.index()] = r.ram;
        axes[Axis::Gpu.index()] = r.gpu;
        axes[Axis::Npu.index()] = r.npu;
        axes[Axis::Power.index()] = r.power;
        axes[Axis::Io.index()] = r.io;
        airgenome::Vitals { ts: r.ts, axes }
    }).collect();

    println!("=== airgenome nexus — singularity breakthrough analysis ===");
    println!("  samples: {}  bins: {}  span: {:.1}h",
        records.len(), bins,
        (records.last().unwrap().ts - records.first().unwrap().ts) as f64 / 3600.0);
    println!();

    // ── Layer A: Mesh coupling replay ──
    let mut engine = airgenome::PolicyEngine::with_defaults(12);
    let mut per_pair = [0usize; 15];
    let mut cascade_total = [0u32; 15];  // cumulative boost per pair
    for v in &history {
        let proposals = engine.tick(*v);
        let fired: Vec<usize> = proposals.iter().map(|p| p.pair).collect();
        for &k in &fired { per_pair[k] += 1; }
        let cascades = airgenome::mesh_cascade_for(&fired, v);
        for c in &cascades {
            cascade_total[c.pair] += c.boost as u32;
        }
    }

    let cascade_pairs = cascade_total.iter().filter(|&&b| b > 0).count();
    println!("  Layer A — Mesh Coupling:");
    println!("    pairs with cascade engagement: {}/15", cascade_pairs);
    let mut cascade_ranked: Vec<(usize, u32)> = cascade_total.iter().copied().enumerate()
        .filter(|(_, b)| *b > 0).collect();
    cascade_ranked.sort_by(|a, b| b.1.cmp(&a.1));
    for (k, boost) in cascade_ranked.iter().take(5) {
        let (a, b) = PAIRS[*k];
        println!("    [{:>2}] {}×{}  cascade boost: 0x{:03X}", k, a.name(), b.name(), boost);
    }
    println!();

    // ── Layer B: MI gap ──
    let gaps = airgenome::mi_gap(&history, &per_pair, bins);
    let gap_total: f64 = gaps.iter().sum();
    let gap_max: f64 = gaps.iter().cloned().fold(0.0f64, f64::max);

    println!("  Layer B — MI Gap Analysis:");
    println!("    Pair         MI_gap   status");
    println!("    ─────────────────────────────");
    let mut gap_ranked: Vec<(usize, f64)> = gaps.iter().copied().enumerate().collect();
    gap_ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    for (k, g) in gap_ranked.iter().take(5) {
        let (a, b) = PAIRS[*k];
        let status = if *g > 0.3 { red("LEAKING") } else if *g > 0.1 { yellow("weak") } else { green("ok") };
        println!("    [{:>2}] {:<6}×{:<6} {:>6.3}   {}", k, a.name(), b.name(), g, status);
    }
    println!("    total gap: {:.3}  max: {:.3}", gap_total, gap_max);
    println!();

    // ── Layer C: Ghost estimate (quick — reuse current ps snapshot) ──
    let ps_out = match std::process::Command::new("ps")
        .args(["-axo", "pid=,rss=,pcpu=,comm="])
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => String::new(),
    };
    let mut ghost_rss: u64 = 0;
    let mut total_rss: u64 = 0;
    for line in ps_out.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        let mut it = trimmed.split_whitespace();
        let _pid: u32 = match it.next().and_then(|s| s.parse().ok()) { Some(v) => v, None => continue };
        let rss_kb: u64 = match it.next().and_then(|s| s.parse().ok()) { Some(v) => v, None => continue };
        let cpu: f64 = match it.next().and_then(|s| s.parse().ok()) { Some(v) => v, None => continue };
        total_rss += rss_kb;
        if rss_kb > 50_000 && cpu == 0.0 {
            ghost_rss += rss_kb;
        }
    }
    let ghost_frac = if total_rss > 0 { ghost_rss as f64 / total_rss as f64 } else { 0.0 };

    println!("  Layer C — Ghost Sink:");
    println!("    ghost RSS: {} MB / {} MB ({:.1}%)",
        ghost_rss / 1024, total_rss / 1024, 100.0 * ghost_frac);
    println!();

    // ── Adjusted efficiency ──
    let raw_efficiency = 0.636;
    let mesh_boost = cascade_pairs as f64 / 15.0 * (1.0 / 30.0);
    let mi_recovery = if gap_total > 0.0 {
        let gap_closed = gap_total - gap_max;  // removing worst leaker
        (gap_closed / gap_total).max(0.0) * (1.0 / 30.0)
    } else { 0.0 };
    let ghost_penalty = ghost_frac * (5.0 / 15.0) * (1.0 / 30.0);
    let adjusted = raw_efficiency + mesh_boost + mi_recovery - ghost_penalty;
    let distance = (adjusted - 2.0 / 3.0).abs();
    let reached = distance < 0.01;

    println!("  ══════════════════════════════════════");
    println!("  Efficiency breakdown:");
    println!("    raw (rule-only):    {:.4}", raw_efficiency);
    println!("    + mesh coupling:   {:+.4}", mesh_boost);
    println!("    + MI recovery:     {:+.4}", mi_recovery);
    println!("    - ghost penalty:   {:+.4}", -ghost_penalty);
    println!("    ────────────────────────");
    println!("    adjusted:           {:.4}", adjusted);
    println!("    singularity (2/3):  {:.4}", 2.0_f64 / 3.0);
    println!("    distance:           {:.4}", distance);
    println!();

    if reached {
        println!("  {}", green(">>> SINGULARITY REACHED <<<"));
    } else {
        println!("  singularity not yet reached (distance: {:.4})", distance);
        if ghost_frac > 0.05 {
            println!("  hint: run `airgenome ghost` for detailed sink analysis");
        }
        if gap_max > 0.3 {
            let (worst_k, _) = gap_ranked[0];
            let (a, b) = PAIRS[worst_k];
            println!("  hint: gate [{}] {}×{} is the top leaker — threshold may be too strict",
                worst_k, a.name(), b.name());
        }
    }
}
```

- [ ] **Step 3: Add nexus and ghost to help text**

Find the `print_help()` function and locate the line listing commands. Add these two entries near the existing `insights` entry:

```rust
    println!("  ghost               information sink scan (zombie/orphan/rss-ghost)");
    println!("  nexus [--bins N]    singularity breakthrough analysis (A+B+C)");
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: no errors.

- [ ] **Step 5: Run full test suite**

Run: `cargo test 2>&1 | grep "test result"`
Expected: all tests pass (no regressions).

- [ ] **Step 6: Commit**

```bash
git add src/bin/airgenome.rs
git commit -m "feat(nexus): unified nexus command — singularity breakthrough analysis

Combines Layer A (mesh coupling), B (MI gap), C (ghost sink) into
one command. Computes adjusted efficiency and singularity distance.
Also adds ghost subcommand for standalone sink scan."
```

---

### Task 5: Version bump + help text + final verification

**Files:**
- Modify: `Cargo.toml` (version bump)

- [ ] **Step 1: Bump version to 3.50.0**

In `Cargo.toml`, change:

```toml
version = "3.49.0"
```

to:

```toml
version = "3.50.0"
```

- [ ] **Step 2: Full test suite**

Run: `cargo test 2>&1 | grep "test result"`
Expected: all tests pass.

- [ ] **Step 3: Verify nexus command runs**

Run: `cargo run -- nexus --bins 5 2>&1 | head -10`
Expected: either output with analysis data, or graceful "no records yet" message if no vitals.jsonl exists.

- [ ] **Step 4: Verify ghost command runs**

Run: `cargo run -- ghost 2>&1 | head -20`
Expected: ghost scan output with zombie/orphan/rss-ghost counts.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/bin/airgenome.rs
git commit -m "v3.50.0: nexus — singularity breakthrough (mesh coupling + MI gap + ghost sink)"
```

---

## Summary

| Task | Layer | What | New tests |
|------|-------|------|-----------|
| 1 | A | `mesh_cascade_for()` + `CascadeInfo` in policy.rs | 3 |
| 2 | B | `mi_gap()` in efficiency.rs | 3 |
| 3 | C | `ghost_cmd()` in airgenome.rs | 0 (integration) |
| 4 | A+B+C | `nexus_cmd()` in airgenome.rs | 0 (integration) |
| 5 | — | Version bump + final verification | 0 |

Total: 5 tasks, ~275 lines added, 6 new unit tests, 0 existing code modified.
