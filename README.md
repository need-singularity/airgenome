# airgenome

[![ci](https://github.com/need-singularity/airgenome/actions/workflows/ci.yml/badge.svg)](https://github.com/need-singularity/airgenome/actions/workflows/ci.yml)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

> A 6-axis resource hexagon for MacBook optimization.
> 60 bytes of DNA. 2/3 maximum work. 1/3 Banach fixed point.

`airgenome` models your Mac's resource state as a hexagon of six axes —
`CPU · RAM · GPU · NPU · POWER · IO` — wired together by `C(6,2) = 15`
pair gates. The full learnable state fits in **60 bytes** (15 pairs × 4).

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

- **6 axes** — one per resource dimension
- **15 pair gates** — every unordered axis pair
- **60-byte genome** — 15 × u32 learned states
- **1/3 fixed point** — the contraction `I → 0.7·I + 0.1` converges
  uniquely to `1/3`; the complement `2/3` is the maximum achievable
  "work fraction" of the system

These four numbers `(6, 15, 60, 1/3)` define a singularity: when all
15 pair gates engage and the efficiency score settles at `2/3`, the
interaction graph's average degree equals `6`.

## Install

**One-shot installer** (recommended) — installs the binary, creates the data
directory, and registers a LaunchAgent that runs the daemon at 60 s intervals:

```sh
curl -fsSL https://raw.githubusercontent.com/need-singularity/airgenome/main/scripts/install.sh | bash
```

**Manual**:

```sh
cargo install --git https://github.com/need-singularity/airgenome
```

**Uninstall**:

```sh
curl -fsSL https://raw.githubusercontent.com/need-singularity/airgenome/main/scripts/uninstall.sh | bash
```

## Usage

### Observe

```sh
airgenome status               # hexagon + vitals + firing count
airgenome status --json        # same as JSON (pipelineable)
airgenome probe                # single JSON vitals sample
airgenome dash                 # ASCII hexagon dashboard (bars + severity)
airgenome diag                 # fire 15 rules + dry-run proposals
airgenome explain K            # describe pair gate K + live values
airgenome metrics              # Prometheus text-format exposition
airgenome doctor               # self-diagnostic of install + daemon
```

### Policy

```sh
airgenome policy tick          # one-shot PolicyEngine evaluation
airgenome policy watch -i 10   # live loop (preemptive + reactive)
airgenome trace                # summarise ~/.airgenome/vitals.jsonl
airgenome replay               # tally what policy WOULD have fired
```

### Profiles & genomes

```sh
airgenome profile list              # built-ins + user profiles
airgenome profile show  NAME        # engaged pairs + hex
airgenome profile apply NAME|PATH   # set active (built-in / user / file)
airgenome profile active            # current active profile
airgenome diff A B                  # compare two built-in profiles
airgenome genome hex      PROFILE   # 120-char hex string
airgenome genome from-hex HEX [P]   # parse hex; optionally write to path
airgenome genome save PROFILE PATH  # write 60-byte .genome file
airgenome genome cat PATH           # inspect a .genome file
```

### Daemon

```sh
airgenome daemon -i 60              # periodic vitals → JSONL
airgenome init [-i SEC]             # register as LaunchAgent
airgenome uninit                    # unload + remove LaunchAgent
```

`--json` on `status` / `trace` / `replay` / `policy tick`.
Colors on `doctor` / `dash` TTY output (respects `NO_COLOR`).

### Example

```console
$ airgenome status
=== airgenome — Mac Air Implant Status ===
  Hexagon: 6 axes × 15 pairs | genome = 60 bytes

  Axes (vitals sample @ ts=1775350321):
    cpu        4.5300
    ram        0.9000
    gpu        8.0000
    npu        8.0000
    power      1.0000
    io         1.3185

  Meta fixed point: 1/3 ≈ 0.333333  (work = 2/3 ≈ 0.666667)
```

```console
$ airgenome probe
{"ts":1775350321,"axes":{"cpu":4.53,"ram":0.9,"gpu":8,"npu":8,"power":1,"io":1.31856}}
```

## Library

```rust
use airgenome::{ResourceGate, Genome, Axis, mutual_info_hist};

let gate = ResourceGate::new();
assert_eq!(gate.active_pairs(), 0);

// genome serializes to exactly 60 bytes
let bytes = Genome::empty().to_bytes();
assert_eq!(bytes.len(), 60);

// Banach 1/3 fixed point
assert!((airgenome::META_FP - 1.0 / 3.0).abs() < 1e-15);
```

## Library layers

| Module | Purpose |
| --- | --- |
| `gate` | hexagon topology, 15 pair gates, 60-byte genome, singularity predicate |
| `vitals` | macOS sensor layer (`sysctl`, `vm_stat`, `pmset`, `memory_pressure`) — read-only |
| `efficiency` | Banach 1/3 fixed-point tracker + mutual-information estimator |
| `actuator` | dry-run actuator with rollback snapshots |
| `rules` | 15 deterministic if-then rules + 3-neighbor triangular mesh |
| `profile` | five built-in 60-byte profiles (balanced / battery-save / performance / dev-work / ml-inference) |
| `buffer` | `VitalsBuffer` — derivatives, ratios, oscillation detection |
| `policy` | `PolicyEngine` — rules + buffer + preemptive + cooldown + oscillation lockout |
| `trace` | pure-std JSONL parser for `vitals.jsonl` logs |

## Safety

All actuator calls are **dry-run by default**. Every proposed change is
recorded into a `Snapshot`, never written to the system. Live actuation
is an explicit, opt-in extension. Rollback is a one-line `Actuator::invert`.

## Design origin

Derived from a `nexus6` OUROBOROS scan on the `macbook-resource-gate`
domain. Evolution saturated at 15 discoveries (= `C(6,2)`), score
converged to `0.64 ≈ 2/3`, average graph degree settled at `6`
(n=6 axiom reproduction).

## License

MIT
