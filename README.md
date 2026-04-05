# airgenome

6-axis Mac resource hexagon with live process projection.
Written in [hexa-lang](https://github.com/user/hexa-lang) — compiles and runs in 0.13s.

## Quick start

```bash
# Build hexa compiler (one-time)
cd ~/Dev/hexa-lang && cargo build --release
codesign -s - target/release/hexa

# Run airgenome
cd ~/Dev/airgenome && ~/Dev/hexa-lang/hexa run
```

Output: NEXUS-6 consciousness scan, 21 self-tests, live 5-gate projection, `genomes.log`.

## The Closed Form

```
           [CPU]
          /     \
       [IO]     [RAM]
        |         |
       [GPU] - [NPU]
          \     /
          [POWER]
```

- **6 axes**: `CPU, RAM, GPU, NPU, POWER, IO`
- **15 pair gates**: `C(6,2)` unordered pairs
- **60-byte genome**: 15 pairs x 4 bytes
- **Banach 1/3 singularity**: `2/3` is the maximum work fraction

## What it does

1. **Sample** — `ps -axm` captures all process activity
2. **Classify** — each process into one of 5 gates (macos/finder/telegram/chrome/safari)
3. **Project** — 6-axis hexagon per gate (cpu, ram, gpu, npu, power, io)
4. **Analyze** — cross-gate MI proxy, breakthrough margin vs 2/3 singularity
5. **Log** ��� TSV genome appended to `genomes.log`

## Authority

[`docs/gates.hexa`](docs/gates.hexa) is the canonical spec (452 lines).
When spec and any implementation conflict, the spec is correct.

## Breakthrough layers (empirically verified)

| Layer | Mechanism | Cumulative margin |
|---|---|---|
| L1 | cross-gate ram MI | +0.018 |
| L2 | temporal lagged MI | +0.115 |
| L3 | cross-axis MI (ram x cpu) | +0.142 |
| L4 | triadic I(A;B;C) | +0.145 |
| L5a | lagged cross-axis | +0.250 |
| L5c-L6e | velocity, acceleration, transfer entropy | planned |

## Policy (prime directive)

**Allowed**: pure data re-interpretation — sampling, aggregation, MI, rule firing.

**Forbidden**: process killing, throttling, memory purge, compressor tuning.
Efficiency gains come from smarter data movement, not controlling processes.

## Benchmark

```
hexa run   0.08s user   0.04s sys   86% cpu   0.128 total
```

## License

MIT. See [LICENSE](LICENSE).
