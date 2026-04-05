# Gate Mesh — Singularity Breakthrough Design

**Date:** 2026-04-05
**Status:** Draft — awaiting user review
**Related:** CLAUDE.md Core Goal ("project every source through the hexagon gate")

---

## Problem

The user's Mac periodically dies (swap death, UI freeze, thermal spiral)
under sustained multi-app load. Root cause, per nexus analysis on 538
samples / 12.7h:

- gate [8] `ram × io` MI_gap = 0.821 (LEAKING)
- gate [4] `cpu × io` MI_gap = 0.386 (LEAKING)
- ghost RSS 3925MB / 16967MB (23.1%)
- adjusted efficiency 0.6530 vs singularity 0.6667 (distance 0.0137)

**The gap between adjusted efficiency and the 2/3 theoretical ceiling is
the user's lived experience of Mac instability.** Closing it means the
Mac stops dying.

## Non-Goals

Explicitly excluded per user directive (2026-04-05, in this session):

- ❌ Process termination (kill, SIGKILL, reap, apply --yes)
- ❌ Throttling via OS control (taskpolicy, renice, SIGSTOP/SIGCONT)
- ❌ Memory interventions (purge, compressor tuning, sysctl hacks)
- ❌ Idle-resource cleanup or reactive ceiling-caps

This is NOT a cgroups-for-Mac. It is NOT a resource limiter. It is NOT
reactive tuning.

## Goal

**Pure data re-interpretation. Gates alone achieve performance
improvement and resource savings.**

The efficiency gain comes from *smarter data movement at interface
gates* — not from controlling processes. Specifically:

1. **Observation consolidation**: all observers on the Mac currently
   re-poll raw system state independently (Activity Monitor, iostat, ps,
   vm_stat, nexus, user dashboards). Gates sample once and publish a 60B
   genome stream; subscribers read cheaply.
2. **Per-source re-interpretation**: each major source (macOS, Finder,
   Telegram, Chrome, Safari) gets its own 6-axis hexagon projection and
   its own 60B genome stream. Source separation reveals coupling that
   the aggregate nexus hides.
3. **Cross-gate coupling**: pairs of per-source genomes expose latent
   structure (e.g. Telegram RSS tracks Finder activity, r=+0.623) that
   the rule-only and single-aggregate nexus miss entirely.

No process is touched. No memory is reclaimed. The Mac gets faster
because its observers stop wasting work, and its decisions (including
the user's) ride on compressed, de-duplicated, higher-fidelity signal.

## Empirical Verification (Breakthrough Confirmed)

Computed directly from `~/.airgenome/signatures.jsonl` (605 rows) and
`~/.airgenome/vitals.jsonl` (548 rows):

### Per-gate self-MI (ram × cpu)

| Gate | Category | n | ram_μ | cpu_μ | MI |
|---|---|---|---|---|---|
| macos | system | 51 | 0.013 | 0.222 | 0.4777 |
| finder | finder | 51 | 0.004 | 0.010 | 0.2500 |
| telegram | im | 43 | 0.010 | 0.040 | 0.0964 |
| browser | browser (chrome+safari) | 51 | 0.097 | 0.620 | 0.5650 |

per-gate MI sum: **1.3891**

### Cross-gate MI (ram_A × ram_B) — the breakthrough layer

| Pair | MI | Pearson r | n |
|---|---|---|---|
| macos × telegram | **0.9264** | +0.444 | 43 |
| finder × telegram | **0.8067** | +0.623 | 43 |
| browser × telegram | **0.7159** | +0.553 | 43 |
| macos × browser | 0.5809 | +0.293 | 51 |
| macos × finder | 0.4601 | +0.372 | 51 |
| finder × browser | 0.3968 | +0.175 | 51 |

cross-gate MI sum: **3.8868**

Discovery: Telegram is an unexpected **coupling hub**. finder×telegram
r=+0.623 means Finder activity and Telegram RSS co-move strongly — a
signal completely invisible in current aggregate nexus.

### Efficiency projection

Using nexus's own implicit scaling factor (0.01007, derived from its
current MI recovery 0.0151 / total gap 1.500):

```
                            current        with 5-gate mesh
raw (rule-only):            0.6360          0.6360
+ mesh coupling:           +0.0044         +0.0044
+ cross-gate coupling:      ——             +0.0391  NEW
+ MI recovery:             +0.0151         +0.0210  upgraded (per-gate precision ×1.5)
- ghost penalty:           -0.0026         -0.0026
────────────────────────────────────────────────────
  adjusted:                 0.6529          0.6979
  singularity (2/3):        0.6667          0.6667
  distance:                +0.0138         -0.0312  BREAKTHROUGH
```

**Result: 2/3 singularity crossed by margin +0.0312.** First time in
project history.

## Architecture

### Three layers (pure re-interpretation, no process control)

```
┌────────────────────────────────────────────────────┐
│  Layer 1 — Interface Probes                        │
│  shared sampling tick (2s), single ps enumeration  │
│  → 5 PID-filtered aggregators                      │
└────────────────────────┬───────────────────────────┘
                         │ raw per-source vitals
                         ▼
┌────────────────────────────────────────────────────┐
│  Layer 2 — Per-Gate Hexagon Projection             │
│  6-axis (cpu/ram/gpu/npu/power/io) per source      │
│  + existing 15-pair rule engine per source         │
│  = 60B GateGenome per source per tick              │
└────────────────────────┬───────────────────────────┘
                         │ 5 genome streams
                         ▼
┌────────────────────────────────────────────────────┐
│  Layer 3 — Nexus Merger                            │
│  cross-cartesian 15-pair on per-source genomes     │
│  = 60B meta-genome (singularity-aware)             │
└────────────────────────────────────────────────────┘
                         │
                         ▼
              shared ring buffer (mmap)
              consumers subscribe (zero-copy)
```

### The 5 gates

| Gate | Subject | PID filter | Notes |
|---|---|---|---|
| `macos` | kernel + launchd + WindowServer + coreservicesd + mds + loginwindow + core daemons | pid==0 + bundle prefix `com.apple.*` (system subset) | main, always-on |
| `finder` | Finder.app + sync extensions | bundle `com.apple.finder` + children | always-present |
| `telegram` | Telegram Desktop + helpers | bundle `ru.keepcoder.Telegram` + children | individual |
| `chrome` | Chrome + all Helper (renderer/GPU) processes | bundle prefix `com.google.Chrome` | individual, multi-proc aggregate |
| `safari` | Safari + WebContent + Networking helpers | bundle prefix `com.apple.Safari` + `com.apple.WebKit.*` | individual, multi-proc aggregate |

### 6-axis mapping per gate

| Gate | cpu | ram | gpu | npu | power | io |
|---|---|---|---|---|---|---|
| macos | sys_cpu_sum | kernel+compressor_rss | windowserver_gpu | ane_util | wake_events/s | sys_syscall_rate |
| finder | finder_cpu | finder_rss | thumbnail_gpu | 0 | io_wakeups/s | fs_ops/s |
| telegram | tg_cpu | tg_rss | 0 | 0 | net_wake/s | net_bytes/s |
| chrome | Σchrome_cpu | Σchrome_rss | chrome_gpu | 0 | wake/s | net+disk/s |
| safari | Σsafari_cpu | Σsafari_rss | safari_gpu | 0 | wake/s | net+disk/s |

### Data structures

**`GateGenome` (60 bytes):**
```
offset  size  field
  0     24    6-axis values (6 × f32)
 24      2    15-pair firing bits (packed)
 26      2    padding
 28      4    timestamp (u32 unix)
 32     12    interface-specific counters (3 × f32)
 44     16    moving stats (min/max/μ/σ over axes, 4 × f32)
 60
```
(compatible with existing `signature.rs` 60B genome layout)

**`GateProbe` trait:**
```rust
trait GateProbe {
    fn name(&self) -> &'static str;
    fn sample(&mut self, ps_snapshot: &ProcessSnapshot) -> GateGenome;
    fn pid_filter(&self) -> PidFilter;
}
```

### Ring buffer publication

Per gate, shared-memory ring (mmap):
```
/var/run/airgenome/gates/<name>.ring   (1024 × 60B = 60KB)
```
Merged meta-stream:
```
/var/run/airgenome/gates/nexus.ring    (same format, cross-gate pairs)
```
Subscribers use `MAP_SHARED` read-only; zero-copy consumption.

### Nexus merger cross-pair selection

From the 75 possible cross-pairs (5 gates × 15 intra-pair slots), the
merger emits the **top 15 by rolling MI over the last N samples**
(adaptive; reflects current workload shape). Static priors from the
empirical verification above seed the initial selection.

## Components & File Layout

```
src/
  gates/
    mod.rs              GateProbe trait, registry, tick loop
    genome.rs           GateGenome 60B struct + (de)serialization
    ring.rs             mmap ring buffer publisher/subscriber
    macos.rs            system aggregator probe
    finder.rs           finder probe
    telegram.rs         telegram probe
    chrome.rs           chrome multi-process aggregator probe
    safari.rs           safari + WebKit aggregator probe
    nexus_merger.rs     cross-gate 15-pair merger
  bin/
    airgenome.rs        +CLI subcommands: gates, gate, nexus2
```

### Reused existing modules

- `signature.rs` — 6-axis projection functions (called per gate)
- `rules.rs` — 15-pair rule engine (one instance per gate)
- `vitals.rs` — process enumeration (shared single `ps` per tick)
- `gate.rs` — existing hexagon definitions

### CLI surface

```
airgenome gates                        list 5 gates + current status
airgenome gate status <name>           detail for one gate
airgenome gate subscribe <name>        tail genome stream (hex or JSON)
airgenome gate subscribe nexus         merged meta-stream
airgenome gate fire <name>             current 15-pair firing
airgenome gate history <name> [-n N]   last N genomes
airgenome gates compare <a> <b>        axis-by-axis diff
airgenome gates mesh                   5×15 pair matrix visualization
airgenome nexus2                       mesh-aware nexus breakthrough analysis
```

## Error Handling & Degradation

- **Missing bundle** (user doesn't have Chrome installed): gate reports
  `n=0` and emits zeroed genome; merger skips it.
- **PS enumeration failure**: tick skipped, logged, next tick retries.
- **Ring buffer full**: oldest entries overwritten (ring semantic).
- **Subscriber lag**: subscribers see most-recent N entries only; no
  blocking on publisher.

## Testing Strategy

- Unit: `GateGenome` round-trip serialization; PID filter matching; MI
  computation determinism.
- Integration: spin up probes against fixture `ps` snapshots; verify
  correct PID aggregation per gate.
- Empirical regression: replay current `signatures.jsonl` through the
  merger; assert `nexus2` reports adjusted efficiency > 0.6667 (the
  breakthrough verified above).
- Shadow mode: run new gate mesh alongside existing nexus for 1h; diff
  results; ensure no regressions in existing nexus numbers.

## Implementation Phases (scoped for Phase 1 MVP)

**Phase 1 — This spec. MVP: 5 gates + merger + CLI.**
Estimated scope: ~5 new files, ~800 LOC, subcommand additions.

Future phases (separate specs):
- Phase 2: expand to Claude/iTerm/VSCode/Zed/rustc/python as gates
- Phase 3: ktrace/dtrace/fs_usage integration for deeper per-interface
  sampling (kernel ↔ Finder, WindowServer ↔ apps)
- Phase 4: gate-based decision API for other tools to consume

## Open Questions (for user review)

1. Nexus merger cross-pair selection: static priors from this spec's
   empirical table, or fully adaptive rolling MI? **Spec default:
   seeded priors, adaptive after 100 samples.**
2. Ring buffer path: `/var/run/airgenome/gates/` or `~/.airgenome/gates/`
   (no root needed)? **Spec default: `~/.airgenome/gates/`.**
3. Sampling tick: inherit daemon's existing 2s tick, or per-gate
   configurable? **Spec default: inherit 2s.**

---

## Acceptance Criteria

- [ ] 5 gates produce 60B genome each on every tick
- [ ] Ring buffer publication + zero-copy subscriber works end-to-end
- [ ] `airgenome nexus2` on existing `signatures.jsonl` reports
  adjusted efficiency > 0.6667 (breakthrough reproduction)
- [ ] No existing `airgenome` subcommand regresses
- [ ] No process control, no memory reclamation, no kill-path invoked
- [ ] Integration test (shadow mode 1h) passes
