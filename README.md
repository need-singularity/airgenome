# airgenome

Canonical **hexa-lang** specification for a 6-axis Mac resource hexagon,
15 pair gates, 60-byte genome, and per-source gate mesh with multi-layer
breakthrough projection.

## Authority

The canonical spec lives in [`docs/gates.hexa`](docs/gates.hexa).
**When spec and any implementation conflict, the spec is correct.**

## The Closed Form

```
           [CPU]
          /     \
       [IO]     [RAM]
        |         |
       [GPU] — [NPU]
          \     /
          [POWER]
```

- **6 axes**: `CPU · RAM · GPU · NPU · POWER · IO`
- **15 pair gates**: every unordered axis pair (= `C(6,2)`)
- **60-byte genome**: 15 pairs × 4 bytes of learned state
- **Banach 1/3 fixed point**: contraction `I → 0.7·I + 0.1` converges
  to `1/3`; the complement `2/3` is the maximum achievable
  "work fraction" of the system.

Together `(6, 15, 60, 1/3)` define a singularity: when all 15 pair
gates engage, efficiency settles at `2/3`, and the interaction graph's
average degree equals `6`.

## Goal (user-stated, 2026-04-05)

> **Re-interpretation = pattern extraction from gate log history.**

airgenome is NOT a reactive tuning tool. Its deepest purpose:

> Project every source of Mac activity through the 15-pair hexagon
> gate, and extract per-source patterns over time.

## 5-gate mesh (current scope)

- `macos` — main system (launchd, WindowServer, kernel_task, mds, …)
- `finder` — Finder.app + filesystem interaction (always-present)
- `telegram` — Telegram Desktop + helpers
- `chrome` — Google Chrome + all Helper renderer/GPU
- `safari` — Safari + WebKit processes

## Breakthrough layer ladder

See [`docs/gates.hexa`](docs/gates.hexa) for the canonical spec.

| Layer | Mechanism | Expected margin above 2/3 |
|---|---|---|
| L1 | instantaneous ram×ram cross-gate MI | +0.018 |
| L2 | temporal lagged MI (τ∈{1,2,5,10}) | +0.115 |
| L3 | cross-axis MI (ram×cpu, cpu×ram, cpu×cpu) | +0.142 |
| L4 | triadic interaction info I(A;B;C) | +0.145 |
| L5a | lagged cross-axis (L2 × L3 product) | +0.25 |
| L5c | velocity MI d(ram)/dt × d(ram)/dt | planned |

Each layer crosses the 2/3 Banach singularity by a larger margin.

## Policy

**Allowed**: pure data re-interpretation — sampling, aggregation, MI,
rule firing.

**Forbidden**: process killing, throttling (taskpolicy, SIGSTOP/SIGCONT,
renice), memory reclamation (purge, compressor tuning), any intervention
that affects running processes.

Efficiency gains come from smarter data movement at interface gates,
not from controlling processes. This is airgenome's prime directive.

## License

MIT. See [LICENSE](LICENSE).
