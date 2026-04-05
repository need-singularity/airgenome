# Changelog

## v1.0.1 — 2026-04-05

Cleanup release.

- Fix `unused import: PAIRS` warning in `rules.rs` (moved to fully-qualified use).
- Fix `unused import: PAIRS` warning in `actuator.rs` test (same treatment).
- Add this CHANGELOG.
- `cargo build --lib` now compiles with zero warnings.

## v1.0.0 — 2026-04-05

First stable release.

- **`airgenome dash`** — bordered ASCII hexagon dashboard showing all 6 axes
  as 10-cell bar charts, all 15 pair gates with per-pair severity
  (ok / wrn / CRI), firing count, work fraction, and a compact severity
  strip (`·` ok, `▒` warn, `█` critical).
- Completes the v1.0 monitoring loop: `status / diag / dash / policy / trace`.

## v0.9.0 — 2026-04-05

- **GitHub Actions CI** — `cargo test` on macos-14 + macos-13 for every
  push/PR; rustfmt + clippy non-blocking.
- **`airgenome init [-i SEC]`** — register the LaunchAgent with the current
  binary's path, no shell script needed.
- **`airgenome uninit`** — unload + remove the plist (data dir preserved).

## v0.8.0 — 2026-04-05

- **`scripts/install.sh`** — one-shot `curl | bash` installer.
- **`scripts/uninstall.sh`** — clean removal, data preserved.
- README rewritten around v0.7 CLI surface.

## v0.7.0 — 2026-04-05

- **`airgenome policy tick`** — seed `PolicyEngine` from vitals.jsonl and
  evaluate one sample.
- **`airgenome policy watch [-i SEC] [-c CAP]`** — continuous live loop,
  prints `REACT` / `PREEMP` tag per proposal, cooldown suppression visible.

## v0.6.0 — 2026-04-05

- **`PolicyEngine`** — wires `VitalsBuffer` + 15 rules + 3 filters into a
  single `tick(Vitals) → Vec<Proposal>` API.
- Per-pair `cooldown_ticks` (default 3) suppresses repeat fires.
- Oscillation lockout: pairs dominated by a flapping axis are suppressed.
- `Proposal` carries `Reason::Reactive | Preemptive`.

## v0.5.0 — 2026-04-05

- **`VitalsBuffer`** — ring buffer of recent vitals samples.
- `derivative(axis)` — `d(axis)/dt` in units per second.
- `preemptive(axis, threshold, sign)` — rate-of-change trigger.
- `ratio(num, denom)` — axis-pair ratio on the latest sample.
- `oscillation_count / oscillating(axis, max_flips)` — sawtooth detection.
- `GateMask` — 15-bit bitmask helper.

## v0.4.0 — 2026-04-05

- **`airgenome trace [--input PATH] [--tail N]`** — summarise a
  `vitals.jsonl` log: means, firing stats, work fraction, battery fraction.
- Pure-std JSONL parser (no new dependencies).

## v0.3.0 — 2026-04-05

- **`airgenome daemon [-i SEC] [-o PATH]`** — periodic vitals loop writing
  JSONL (default `~/.airgenome/vitals.jsonl`).
- **`airgenome profile active`** — read the active genome and match it
  against built-in profiles.
- **`airgenome diff A B`** — per-pair genome diff between two profiles.

## v0.2.0 — 2026-04-05

- 15 deterministic pair-gate rules (one per hexagon edge).
- Triangular mesh: `neighbors(k) = {k+1, k+5, k+11} mod 15` → 45 directed
  edges. Reproduces the `Δedges = 3·Δnodes` invariant from the nexus6
  evolution.
- Five built-in profiles: `balanced`, `battery-save`, `performance`,
  `dev-work`, `ml-inference`.
- New CLI: `rules`, `diag`, `profile list / show / apply`.

## v0.1.0 — 2026-04-05

Initial release.

- 6-axis hexagon (`cpu · ram · gpu · npu · power · io`) + 15 pair gates
  (= `C(6,2)`).
- 60-byte `Genome` (`15 × u32`).
- Banach 1/3 fixed-point contraction (`I → 0.7·I + 0.1`).
- `ResourceGate::singularity_reached` — full engagement + efficiency=2/3
  + avg_degree=6.
- macOS vitals layer: `sysctl`, `vm_stat`, `pmset`, `memory_pressure`.
- Dry-run `Actuator` with rollback snapshots.
- Histogram mutual-information estimator.
- 29 tests.

Origin: nexus6 OUROBOROS scan on the `macbook-resource-gate` domain
(15 discoveries saturation, work fraction → 2/3, avg degree → 6).
