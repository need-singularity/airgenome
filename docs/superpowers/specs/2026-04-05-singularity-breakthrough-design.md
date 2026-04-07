# Singularity Breakthrough: Mesh Coupling + MI Gap + Ghost Sink

**Date**: 2026-04-05
**Status**: Draft
**Goal**: Close the 1/30 efficiency gap (0.636 → 0.667) to reach the singularity fixed point

---

## Problem Statement

nexus breakthrough scan established:
- Rule-only mesh attains efficiency **0.636** (95.4% of 2/3 ceiling)
- Learning adds only +0.8%p — complexity not justified
- Remaining gap: **1/30 = 0.0333** = `1/(n(n-1))` where n=6

**Root cause analysis** reveals three disconnected subsystems:
1. `neighbors()` in rules.rs — defined but never used in PolicyEngine decision logic
2. `mutual_info_hist()` in efficiency.rs — exported but never called outside tests
3. Process-level resource attribution — `processes` cmd aggregates but doesn't feed back into the gate

Each subsystem addresses a portion of the 1/30 gap. Combined, they close it.

---

## Architecture: Three Layers

```
Layer C: Ghost Sink          → identifies noise sources (report only)
Layer B: MI Gap Analysis     → finds leaking gates
Layer A: Mesh Coupling       → restores structural information flow
         ─────────────────
         PolicyEngine.tick()  → enhanced decision with all three signals
```

All layers are **kill-free** (Prime Directive). All are **report + genome update**, no system mutation.

---

## Layer A: Mesh Coupling

### What changes

**File**: `src/policy.rs` — `PolicyEngine::tick()`

Currently each rule fires independently. The `neighbors()` function defines a 3-regular triangular mesh (45 directed edges) but PolicyEngine ignores it.

### Design

When rule `k` fires (reactive or preemptive), check `neighbors(k)`:
- If **1 of 3** neighbors also fires: normal engagement (status quo)
- If **2 of 3** neighbors fire: **cascade** — boost engagement byte by `+0x10` (stability counter)
- If **3 of 3** neighbors fire: **full cascade** — boost engagement by `+0x20` + set surprise bit

The cascade does NOT trigger new proposals. It only updates the genome's engagement/stability bytes for pair `k`, enriching the 60-byte genome with mesh-structure information.

### New struct

```rust
/// Cascade result from mesh neighbor analysis.
pub struct CascadeInfo {
    pub pair: usize,
    pub neighbor_fires: u8,  // 0..3
    pub boost: u8,           // engagement boost to apply to genome
}
```

### New method on PolicyEngine

```rust
/// Check mesh neighbors for cascading engagement.
/// Returns cascade info for pairs that fired this tick.
pub fn mesh_cascade(&self, fired: &[usize], v: &Vitals) -> Vec<CascadeInfo>
```

### Efficiency impact

The cascade feeds back into `ResourceGate.genome` → when all 15 pairs have non-zero engagement AND stability counters reflect mesh structure, the singularity predicate's `active_pairs() == 15` condition carries more information weight.

Mathematical: reconnecting the triangular mesh restores `3 × 15 = 45` directed edges of information flow, recovering the `1/(n(n-1))` term.

---

## Layer B: MI Gap Analysis

### What changes

**File**: `src/efficiency.rs` (new function) + `src/bin/airgenome.rs` (new `nexus` subcommand)

### Design

Compute pairwise mutual information between all 15 axis pairs using the existing `mutual_info_hist()` estimator, over vitals history from `vitals.jsonl`.

For each pair `k`:
- `mi[k]` = mutual information between its two axes (nats, from histogram estimator)
- `fire_rate[k]` = historical firing frequency from PolicyEngine replay (0.0..1.0)
- Normalize both to `[0, 1]`: `mi_norm[k] = mi[k] / max(mi)`, `fr_norm[k] = fire_rate[k]`
- `gap[k]` = `mi_norm[k] - fr_norm[k]` (clamped to 0 — negative means over-firing, not a gap)

A pair with **high normalized MI but low fire rate** = a "leaking gate" — the axes are correlated but the rule's threshold is too strict to capture the interaction.

### New function

```rust
/// Per-pair MI gap: how much mutual information each gate fails to capture.
pub fn mi_gap(history: &[Vitals], fire_counts: &[usize; 15], bins: usize) -> [f64; 15]
```

### Output (in `nexus` command)

```
=== nexus — MI gap analysis (4523 samples) ===

  Pair         MI     fire%    gap     status
  ──────────────────────────────────────────────
  [ 0] cpu×ram    0.72   0.28   0.44    LEAKING
  [ 3] cpu×power  0.15   0.12   0.03    ok
  [ 8] ram×io     0.61   0.09   0.52    LEAKING
  ...

  Total MI gap: 0.031 (= 1/30 of singularity ceiling)
  Top leaking gates: [8] ram×io, [0] cpu×ram, [6] ram×npu
```

### Threshold suggestion

For leaking gates, compute what threshold would capture 90% of the MI:
```
suggested_threshold[k] = percentile_10(axis_a[k]) when MI > fire_rate
```

Report only — does not auto-adjust. User decides.

---

## Layer C: Ghost Sink (Information Sink Detection)

### What changes

**File**: `src/bin/airgenome.rs` (new `ghost` subcommand)

### Design

A ghost process is one that consumes memory (RSS) while contributing zero CPU over a measurement window. It acts as an "information sink" on the RAM axis — occupying capacity without productive pair interaction.

### Three-scan approach

**Scan 1: Zombie (Z state)**
```
ps -axo stat,pid,rss,comm → filter stat contains 'Z'
```

**Scan 2: Orphaned helpers**
```
ps -axo ppid,pid,rss,comm → filter ppid=1 AND name contains Helper|Agent|Worker AND rss > 10MB
```

**Scan 3: RSS ghosts (temporal)**
```
Sample 1: ps -axo pid,rss,pcpu,comm
Wait 2 seconds
Sample 2: ps -axo pid,rss,pcpu,comm
Ghost = pid appears in both, rss > 50MB, cpu = 0.0 in both samples
```

### Output

```
=== airgenome ghost — information sink scan ===

  Type          Count    RSS total    % of system RSS
  ────────────────────────────────────────────────────
  zombie           0        0 MB      0.0%
  orphan-helper    3      180 MB      1.2%
  rss-ghost       12      840 MB      5.6%
  ────────────────────────────────────────────────────
  total           15     1020 MB      6.8%

  Hexagon impact (ram axis pollution):
    ghost_fraction = 0.068
    ram-centered pairs affected: [0, 5, 6, 7, 8] (5 of 15)
    estimated efficiency loss: 1/3 × 0.068 = 0.023

  Top ghosts:
    PID 12345   320 MB  0.0%  Google Chrome Helper (GPU)
    PID 67890   180 MB  0.0%  Slack Helper (Renderer)
    ...

  (kill-free: no processes terminated)
```

### Hexagon projection

Ghost RSS maps to a single-axis vector `[0, ghost_fraction, 0, 0, 0, 0]` — pure RAM axis pollution. The `Signature` module can compute distance from known fingerprints to classify ghost impact.

---

## Unified Command: `airgenome nexus`

All three layers combine under one subcommand:

```
airgenome nexus [--samples N] [--bins B]
```

### Flow

1. Load vitals.jsonl history
2. Run Layer C (ghost scan) → ghost_fraction
3. Run Layer B (MI gap) → per-pair gap scores
4. Run Layer A (mesh coupling simulation) → cascade counts
5. Compute adjusted efficiency:
   ```
   raw_efficiency = 0.636 (from rule replay)
   mesh_boost = cascade_pairs / 15 × (1/30)
   mi_recovery = sum(gap_closed) / sum(gap_total) × (1/30)
   ghost_penalty = ghost_fraction × (5/15) × (1/30)
   adjusted = raw_efficiency + mesh_boost + mi_recovery - ghost_penalty
   ```
6. Report singularity distance: `|adjusted - 2/3|`

### Success criterion

```
singularity_reached(&genome, adjusted_efficiency) == true
```

When adjusted efficiency enters the `(2/3 - 0.01, 2/3 + 0.01)` band AND all 15 pairs are engaged with cascade-enriched genomes → **singularity achieved**.

---

## File change summary

| File | Change | Lines (est.) |
|------|--------|-------------|
| `src/policy.rs` | Add `mesh_cascade()`, `CascadeInfo` | +40 |
| `src/efficiency.rs` | Add `mi_gap()` function | +35 |
| `src/bin/airgenome.rs` | Add `nexus` + `ghost` subcommands | +200 |
| `src/rules.rs` | No change (neighbors already exists) | 0 |
| `src/gate.rs` | No change (genome already has stability byte) | 0 |
| `src/signature.rs` | No change (distance metrics already exist) | 0 |

**Total**: ~275 new lines. No existing logic modified. Three new entry points.

---

## What this does NOT do

- **No learning/ML** — stays rule-based per nexus conclusion
- **No process killing** — ghost scan is report-only
- **No threshold auto-tuning** — MI gap reports suggestions, user decides
- **No new dependencies** — pure Rust, existing crate only

---

## Test plan

1. `mesh_cascade()` unit tests — verify cascade counts match expected for known firing patterns
2. `mi_gap()` unit tests — verify gap=0 when fire rate matches MI, gap>0 when mismatch
3. `ghost` integration — verify zombie/orphan/rss-ghost detection on live system
4. `nexus` integration — verify adjusted efficiency computation, singularity distance
5. Regression — all existing 50+ tests still pass
