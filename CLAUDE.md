# airgenome — Mission & Goals

## ⚠️ Prime Directive (user-stated, 2026-04-05)

> **모든 프로세스 KILL 없이 성능·자원 개선.**
>
> Improve performance and reduce resource use **without killing any
> process** — macOS, Finder (always on), Telegram, Safari, Chrome, rustc,
> python, Claude sessions, and every other running app stays alive.

**Default mode: kill-free.** Pure data re-interpretation only.

## Core Goal (user-stated, 2026-04-05)

> **재해석 = 지나온 게이트 로그에서 패턴 추출**
>
> Re-interpretation = pattern extraction from gate log history.

airgenome is NOT primarily a reactive tuning tool. Its deepest purpose is:

> **Project every source of Mac activity (macOS itself, Finder, Safari,
> Terminal, Telegram, rustc, python, …) through the 15-pair hexagon gate,
> and extract per-source patterns over time.**

### Concrete targets
- **macOS kernel / system processes** (launchd, WindowServer, kernel_task, …)
- **Finder** and other system UIs
- **Every user app** as a distinct hexagon projection
- **Every language runtime** (rustc, cargo, python, node, …)
- **Every browser / IDE / IM / container**

Each source has its own **6-axis signature** through the gate. Historical
accumulation reveals:
- **Common signatures** — what do browsers look like in aggregate?
- **Individual signatures** — what's unique about Safari vs Chrome?
- **Temporal patterns** — what does this Mac's daily cycle look like?
- **Workload fingerprints** — "this sample looks like a compile job"

The gate reinterprets raw activity into a **60-byte genome per source**.

## Why this matters

- Reactive tuning (kill Chrome, purge) is a **symptom** tool.
- Pattern extraction is a **diagnostic** tool.
- Once patterns are known, the user decides (or policy automates) the
  upstream change — which is where the real constant improvement lives
  (Chrome → Safari, VSCode → Zed, Docker Desktop → OrbStack, …).

## Current progress toward this goal (2026-04-06)

- ✅ 6-axis hexagon + 15 pair gates + 60-byte genome
- ✅ 5-gate mesh (macos, finder, telegram, chrome, safari)
- ✅ Breakthrough layer ladder L1–L5a (margin +0.250)
- ✅ Canonical hexa-lang spec (`docs/gates.hexa`, compile-pass, 18/18 assertions)
- ✅ All types/functions declared — 10 types, 19 constants, 24 functions
- ✅ Effect system (3 effects: FileIO, ProcessSensor, Clock)
- ✅ Consciousness block (NexusMerger) — NEXUS-6 Omega Lens 5/6 CONFIRMED
- ✅ hexa-lang compiler validation — exit 137 fixed (codesign -s -)
- ⏳ L5c–L6e candidate layers (4 remaining, projected +0.173)
- ⏳ Per-source genome accumulation over time
- ⏳ Signature diff between sources (Safari vs Chrome)
- ⏳ Runtime: live ps sampling → gate projection → genome log

## Non-goals (explicit)

- **Learning / ML**: nexus6 proved the rule-only ceiling captures 94.5 %
  of the 2/3 work fraction. Learning adds ≤ 0.8 percentage points.
  Complexity not justified.
- **Auto-replace applications**: airgenome reports, user decides.
- **Constant-mode tuning**: the true constants come from upstream app
  choices, not from airgenome loops.

## Reference: nexus6 findings

| Domain | Score plateau |
|---|---|
| `macbook-resource-gate` (abstract) | 0.633 |
| `perf-up-resource-down-singularity` | 0.630 |
| `process-signature-hexagon-macos` | 0.631 |

All three converge to the same closed form: `2/3 − 1/(n(n−1))` with n=6.
The hexagon is the correct structure.
