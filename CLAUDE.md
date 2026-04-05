# airgenome — Mission & Goals

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

## Current progress toward this goal (2026-04-05)

- ✅ 6-axis hexagon + 15 pair gates + 60-byte genome
- ✅ Real-time vitals sampling (daemon)
- ✅ Historical vitals log (`vitals.jsonl`)
- ✅ PolicyEngine replay over history
- ✅ `processes` command (per-app RSS / CPU categorization) — v3.21+
- ✅ `insights` command (per-pair firing counts, hourly patterns, profile
  recommendation from log) — v3.21+
- ⏳ Per-process hexagon projection (RSS/CPU per app → 6-axis vector)
- ⏳ Per-source genome accumulation over time
- ⏳ Signature diff between sources (Safari vs Chrome)
- ⏳ Anomaly detection (this sample doesn't match any known signature)

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
