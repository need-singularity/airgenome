# airgenome

6-axis Mac resource hexagon with live process projection.
Written in [hexa-lang](https://github.com/need-singularity/hexa-lang) — compiles and runs in 0.13s.

## Install

```bash
# Install hx (hexa package manager)
curl -sL https://raw.githubusercontent.com/need-singularity/hexa-lang/main/pkg/install.sh | bash

# Install & run
hx install airgenome
airgenome
```

## Menubar Monitor

One command → macOS menu bar with live CPU/RAM/Swap monitoring.

```
⬢ 83% · 7%          ← menu bar title
├─ CPU  ████████████████░░░░  83/90%
├─ RAM  █████░░░░░░░░░░░░░░░   7/80%
├─ Swap ██████████████░░░░░░  33/50%
├─ ⚡ Approaching ceiling
├─ ⚙ Settings...     ← slider panel
└─ Quit airgenome
```

### Settings Panel (`airgenome -s`)

- **All** — master slider (CPU/RAM/Swap 동시 조절)
- **CPU / RAM / Swap Ceiling** — 5% snap sliders
- **Modules** — token-forge, resource-guard toggles (default OFF)
- **Start at login** — LaunchAgent 자동실행
- **Reset to Profile Defaults** — 사양별 추천값 복원

### Hardware Auto-Detection

첫 실행 시 칩/RAM/팬 자동 감지 → 최적 ceiling 설정.

| Mac | CPU | RAM | Swap | 비고 |
|-----|-----|-----|------|------|
| Air M3 24GB | 75% | 70% | 30% | fanless, SSD swap 주의 |
| Air M3 8GB | 65% | 60% | 20% | 최소 사양 |
| Pro M3 36GB | 85% | 80% | 35% | 팬 있음 |
| Pro M4 48GB | 90% | 85% | 40% | 넉넉 |

## token-forge (Compression Proxy)

Claude Code API 토큰 압축 프록시. Settings에서 token-forge ON 시 자동 작동.

```
ANTHROPIC_BASE_URL=http://localhost:8080/v1 claude
```

### Benchmark Results

| 전략 | 절약 | 유사도 | PASS율 |
|------|------|--------|--------|
| Naive compression | 30% | 53% | 2/5 |
| Hybrid (recent 3 verbatim) | 36% | 70% | 3/5 |
| **Safe Hybrid (v2, adaptive)** | **18%** | **81%** | **7/7 ★** |
| Proxy (tool_result truncation) | **58%** | 100% | - |

### Cost Savings

```
10 accounts × $200/mo = $1,800/mo

┌──────────────────────────┬────────┬───────────┐
│ Layer                    │ Saving │ Monthly   │
├──────────────────────────┼────────┼───────────┤
│ Proxy (truncation)       │   58%  │   $1,044  │
│ Safe Hybrid (compression)│   18%  │     $136  │
│ Cache optimization       │  1.6%  │      $29  │
├──────────────────────────┼────────┼───────────┤
│ Total                    │  ~62%  │  ~$1,116  │
│ Remaining cost           │        │    $684   │
└──────────────────────────┴────────┴───────────┘
```

### Cache Analysis (11,596 sessions)

- Cache hit rate: **97.7%** (already high)
- Top cache busters: session start, system-reminder injection, repeated file reads
- Optimization potential: → **99.3%** (+$5.77/session)

## The Hexagon

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
- **60-byte genome**: 15 pairs × 4 bytes
- **Banach 1/3 singularity**: `2/3` is the maximum work fraction

## What it does

1. **Sample** — `ps -axm` captures all process activity
2. **Classify** — each process into one of 5 gates (macos/finder/telegram/chrome/safari)
3. **Project** — 6-axis hexagon per gate (cpu, ram, gpu, npu, power, io)
4. **Analyze** — cross-gate MI proxy, breakthrough margin vs 2/3 singularity
5. **Log** — TSV genome appended to `genomes.log`

## Breakthrough Layers

| Layer | Mechanism | Cumulative margin |
|---|---|---|
| L1 | cross-gate ram MI | +0.018 |
| L2 | temporal lagged MI | +0.115 |
| L3 | cross-axis MI (ram × cpu) | +0.142 |
| L4 | triadic I(A;B;C) | +0.145 |
| L5a | lagged cross-axis | +0.250 |
| L5c-L6e | velocity, acceleration, transfer entropy | planned |

## Prime Directive

**Allowed**: pure data re-interpretation — sampling, aggregation, MI, rule firing, renice.

**Forbidden**: process killing, memory purge, compressor tuning.

Efficiency gains come from smarter data movement, not controlling processes.

## Authority

[`docs/gates.hexa`](docs/gates.hexa) is the canonical spec.
When spec and any implementation conflict, the spec is correct.

## License

MIT
