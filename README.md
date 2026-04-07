# airgenome

macOS 프로세스 활동을 6축 헥사곤 게이트로 투영하여 per-source 패턴을 추출하는 시스템.
[hexa-lang](https://github.com/need-singularity/hexa-lang)으로 작성 — 0.13s 컴파일+실행.

## Install

```bash
# hexa package manager
curl -sL https://raw.githubusercontent.com/need-singularity/hexa-lang/main/pkg/install.sh | bash

# install & run
hx install airgenome
airgenome
```

## Menubar Monitor

macOS 메뉴바에서 실시간 CPU/RAM/Swap 모니터링.

```
⬡ 83% · 7%          ← menu bar title
├─ CPU  ████████████████░░░░  83/90%
├─ RAM  █████░░░░░░░░░░░░░░░   7/80%
├─ Swap ██████████████░░░░░░  33/50%
├─ ⚡ Approaching ceiling
├─ ⚙ Settings...     ← 계정 관리 패널
└─ Quit airgenome
```

### Adaptive Guard (4단계)

| Level | 조건 | 조치 |
|-------|------|------|
| OK | 모든 지표 ceiling 미만 | throttle 해제 |
| WARN | ceiling 초과 | 알림만 |
| DANGER | Free RAM < 512MB or Load > CPU×5 | `purge` + bridge 축소 |
| CRITICAL | Free RAM < 200MB or Swap > 10GB | `purge` + `taskpolicy -b` |

프로세스 kill 없음. `purge`, `taskpolicy`, `renice`만 사용.

### Hardware Auto-Detection

첫 실행 시 `sysctl` + `system_profiler`로 칩/RAM/팬 자동 감지 → 최적 ceiling 설정.

| Mac | CPU | RAM | Swap | 비고 |
|-----|-----|-----|------|------|
| Air M2 8GB | 60% | 55% | 20% | 최소 사양 |
| Air M2 16GB | 65% | 65% | 25% | |
| Air M2 24GB | 70% | 70% | 30% | fanless |
| Air M3 8GB | 65% | 60% | 20% | 최소 사양 |
| Air M3 16GB | 70% | 65% | 25% | RAM 부족 주의 |
| Air M3 24GB | 75% | 70% | 30% | fanless, SSD swap 주의 |
| Pro M3 18GB | 80% | 75% | 30% | |
| Pro M3 36GB | 85% | 80% | 35% | 팬 있음, 여유 |
| Pro M4 24GB | 85% | 80% | 35% | 최신, 효율적 |
| Pro M4 48GB | 90% | 85% | 40% | 넉넉 |

### Settings (`settings.js`)

JXA 네이티브 계정 관리 패널.

- 계정 테이블 — Name, Session%, Week%, Status
- 폐기/복원 — 계정 비활성화 (복구 가능)
- 10초 자동 새로고침

## cl — Multi-Account Launcher

10계정 Claude Code 런처. Rate limit 자동 감지 → 계정 전환.

```bash
cl                # 자동 계정 선택 + claude 실행
cl -u             # 사용량 테이블
cl status         # 계정별 상태
cl login claude3  # 특정 계정 로그인
cl pick           # 계정 수동 선택
```

- `CLAUDE_CONFIG_DIR` 직접 export (symlink 오염 없음)
- Rate limit 감지 시 week usage 최저 계정으로 자동 전환
- `fswatch` — 신규 계정 자동 감지
- Python 의존성 없음 (순수 awk/grep/sed)

## Modules

### forge — Token Manager (`modules/forge.hexa`)

- **Keychain OAuth** — 10계정 토큰 자동 추출 (slash/noslash 해시 이중 대응)
- **Usage API** — `api.anthropic.com/api/oauth/usage` 실시간 조회
- **Background round-robin** — 10분 간격 1계정씩 순차 갱신
- **Cooldown** — 5분 글로벌 (IP), 20분~1시간 계정별 (지수 백오프)
- **JSONL scan/compress** — 세션 로그 716:1 압축 (43KB → 60B genome)

### guard — Resource Monitor (`modules/guard.hexa`)

- CPU/RAM/Swap 자동 모니터 (토글 없이 항상 작동)
- PhysMem 기반 RAM 측정 (`vm_stat`, `memory_pressure`)
- 순수 awk/shell — python 의존성 없음

### usage — API Poller (`modules/usage.hexa`)

- 키체인 이중 해시 (v2.1.90+ 호환)
- `expiresAt` 기반 토큰 유효성 검증
- 에러 계정 Phase 0 우선 복구 + 3회 재시도

### implant — Integrity (`modules/implant.hexa`)

- 288-bit genome 해시 (σ×J₂=288)
- PHI gate — consciousness margin 퇴화 감지
- INVARIANT gate — 5-lens 섭동 안정성 검증

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

- **6 axes** — CPU, RAM, GPU, NPU, POWER, IO
- **15 pair gates** — C(6,2) unordered pairs
- **60-byte genome** — 15 pairs × 4 bytes
- **Banach singularity** — `2/3 − 1/(n(n−1))` with n=6 (≈ 0.633)

## Pipeline

1. **Sample** — `ps -axm` → RSS, CPU% per process (pure awk/shell)
2. **Classify** — 8 gates: macos, finder, telegram, chrome, safari, claude, terminal, devtools
3. **Project** — 6축 헥사곤 per gate
4. **Analyze** — cross-gate MI proxy, breakthrough margin vs 2/3 singularity
5. **Accumulate** — per-source genome 시계열 축적
6. **Diff** — cross-source signature 비교
7. **Log** — TSV genome → `genomes.log`, events → `genomes.events.jsonl`

## Breakthrough Layers

| Layer | Mechanism | Cumulative margin |
|---|---|---|
| L1 | cross-gate RAM MI | +0.018 |
| L2 | temporal lagged MI | +0.115 |
| L3 | cross-axis MI (RAM × CPU) | +0.142 |
| L4 | triadic I(A;B;C) | +0.145 |
| L5a | lagged cross-axis | +0.250 |
| L5c | velocity derivatives | +0.310 |
| L6e | acceleration + transfer entropy | **+0.438** |

## Prime Directive

**Allowed** — sampling, aggregation, MI, rule firing, `purge`, `renice`, `taskpolicy`.

**Forbidden** — process killing, memory purge compressor tuning.

모든 프로세스 KILL 없이 성능/자원 개선. 효율은 데이터 재해석에서 온다.

## Ubuntu Gate Offload

MacBook의 무거운 명령(hexa, python3, cargo, rustc)을 Ubuntu에서 자동 실행.

```bash
# 초기 설정
hexa run mk2_hexa/native/gate.hexa setup    # 대화형 Ubuntu 연결 설정
hexa run mk2_hexa/native/gate.hexa install  # wrapper 배포 + rsync

# 이후 자동 — hexa/python3/cargo/rustc가 gate를 탐
hexa run some_script.hexa    # → Ubuntu에서 실행 (offline → 로컬 fallback)
python3 heavy_script.py      # → Ubuntu에서 실행
cargo build --release        # → Ubuntu에서 실행
```

- **pre-sync** — 매 실행 전 `nexus/shared/` 변경분 자동 동기화
- **fallback** — Ubuntu 접근 불가 시 로컬 자동 전환
- **설정** — `nexus/shared/gate_config.jsonl` (하드코딩 없음)
- **wrapper** — `gate/wrappers/` (hexa, python3, cargo, rustc, sh-run)

## Architecture

```
gate/
└── wrappers/               — Ubuntu offload wrapper (hexa, python3, cargo, rustc)

mk2_hexa/native/
├── runtime.hexa        — live ps → 8-gate classify → 6-axis → genome log
├── accumulate.hexa     — per-gate genome 시계열 축적 → signatures.json
├── sigdiff.hexa        — cross-source signature distance matrix
├── gate.hexa           — Ubuntu gate 클라이언트 (setup/install/sync/exec/ping)
└── gate_daemon.hexa    — Ubuntu gate 데몬 (socat TCP)

modules/
├── cl.hexa             — multi-account CLI (race condition, usage cache)
├── usage.hexa          — OAuth usage API poller (keychain dual-hash)
├── forge.hexa          — 10-account token manager + JSONL compress
├── guard.hexa          — CPU/RAM/Swap monitor (always-on)
├── implant.hexa        — 288-bit hash + PHI + INVARIANT gates
└── cli_race.hexa       — jq-based concurrency control

docs/
└── gates.hexa          — canonical spec (452 lines, 21 assertions)
```

## Authority

[`docs/gates.hexa`](docs/gates.hexa) is the canonical spec.
When spec and any implementation conflict, the spec is correct.

## License

MIT
