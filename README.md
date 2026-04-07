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
- **Modules** — token-forge toggle (default OFF)
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

## token-forge (10-account Manager)

Claude Code 10계정 관리자. keychain OAuth 추출, usage API, 백그라운드 라운드 로빈 갱신.

```bash
# Usage 확인
cl -u

# 계정별 사용량 테이블
cl
```

### Features

- **Keychain OAuth extraction** — macOS keychain에서 토큰 자동 추출
- **Usage API** — 계정별 사용량 실시간 조회
- **Background round-robin refresh** — 10분 간격 1계정씩 순차 갱신
- **JSONL scan/compress** — 세션 로그 압축
- **JXA menubar** — 네이티브 macOS 메뉴바 통합

## resource-guard (Auto Monitor)

CPU/RAM/Swap 자동 모니터. 토글 없이 무조건 자동 작동.
순수 awk/shell 기반 — python 의존성 없음.

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

1. **Sample** — `ps -axm` captures all process activity (pure awk/shell)
2. **Classify** — each process into one of 5 gates (macos/finder/telegram/chrome/safari)
3. **Project** — 6-axis hexagon per gate (cpu, ram, gpu, npu, power, io)
4. **Analyze** — cross-gate MI proxy, breakthrough margin vs 2/3 singularity
5. **Accumulate** — per-source genome history (`accumulate.hexa`)
6. **Diff** — signature comparison between sources (`sigdiff.hexa`)
7. **Log** — TSV genome appended to `genomes.log`

## Runtime Loop

`mk2_hexa/native/runtime.hexa` — live ps sampling → gate projection → genome log.
연속 실행으로 per-source 6-axis signature를 축적하고 temporal pattern을 추출.

## Breakthrough Layers

| Layer | Mechanism | Cumulative margin |
|---|---|---|
| L1 | cross-gate ram MI | +0.018 |
| L2 | temporal lagged MI | +0.115 |
| L3 | cross-axis MI (ram × cpu) | +0.142 |
| L4 | triadic I(A;B;C) | +0.145 |
| L5a | lagged cross-axis | +0.250 |
| L5c | velocity derivatives | +0.310 |
| L6e | acceleration + transfer entropy | **+0.438** |

## Prime Directive

**Allowed**: pure data re-interpretation — sampling, aggregation, MI, rule firing, renice.

**Forbidden**: process killing, memory purge, compressor tuning.

Efficiency gains come from smarter data movement, not controlling processes.

## Architecture

```
mk2_hexa/native/
├── runtime.hexa      — live sampling → gate projection → genome log
├── accumulate.hexa   — per-source genome history accumulation
└── sigdiff.hexa      — cross-source signature diff
```

## Authority

[`docs/gates.hexa`](docs/gates.hexa) is the canonical spec.
When spec and any implementation conflict, the spec is correct.

## License

MIT
