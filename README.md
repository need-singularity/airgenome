# airgenome

macOS 프로세스를 6축 육각 투영(CPU/RAM/GPU/NPU/Power/IO)으로 변환하는 시스템 모니터.
모든 코드는 [hexa-lang](https://github.com/need-singularity/hexa-lang)으로 작성. 0.13s 컴파일+실행.

## Prime Directive

> 모든 프로세스 KILL 없이 성능/자원 개선. 효율은 데이터 재해석에서 온다.

kill/SIGTERM/SIGKILL 절대 금지. renice, taskpolicy, purge(user-space)만 허용.

---

## 기능 영역

airgenome은 5개 영역으로 구성된다:

| 영역 | 핵심 파일 | 설명 |
|------|----------|------|
| **6축 코어** | `src/core.hexa` | 60바이트 게놈 시그니처 생성 |
| **시스템 모니터** | `run.hexa`, `sampler.hexa`, `menubar.hexa`, `settings.hexa` | macOS 메뉴바 실시간 모니터 |
| **Claude Code 관리** | `cl`, `modules/cl.hexa`, `modules/usage.hexa` | 10계정 런처 + Usage API |
| **Ubuntu 오프로드** | `mk2_hexa/native/gate.hexa`, `gate_daemon.hexa`, `ubu_monitor.hexa` | Mac ↔ Ubuntu 원격 자원 공유 |
| **분석 엔진** | `mk2_hexa/native/*.hexa` | 패턴 추출, 이상 탐지, 예측, QoS |
| **Cascade 방어** | `modules/detectors/cascade_detector.hexa`, `predictive_throttle.hexa` | RAM→Swap→Disk 캐스케이드 예측+선제 쓰로틀 |
| **Ubuntu GPU Workers** | `ubu_workers/*.hexa` | Python→hexa 포팅 진행중 (tensor/matmul/WGSL codegen, torch 무의존) |

---

## 1. 6축 코어

```
         [CPU]
        /     \
     [IO]     [RAM]
      |         |
     [GPU] - [NPU]
        \     /
        [POWER]
```

- **6 axes** — CPU, RAM, GPU, NPU, POWER(Swap), IO
- **15 pair gates** — C(6,2) 비순서 쌍
- **60-byte genome** — 15 pairs x 4 bytes

`src/core.hexa`가 `ps`/`top`/`vm_stat`으로 6축 값을 샘플링하고, 심각도(Ok/Warn/Critical)를 판정한다. 하드 제한(setrlimit)과 소프트 쓰로틀(적응적 배치 축소)을 결합한 리소스 가드를 제공한다.

---

## 2. 시스템 모니터

### Menubar

```
⬡ 83% · 7%          ← menu bar title
├─ CPU  ████████████████░░░░  83/90%
├─ RAM  █████░░░░░░░░░░░░░░░   7/80%
├─ Swap ██████████████░░░░░░  33/50%
├─ ↓ Save  CPU -12%  RAM -8%  (≈10% 절감)
├─ ● Ubuntu  load=2.1  ↑3jobs
│  ├─ CPU  ████░░░░░░░░░░░░  25%
│  ├─ RAM  ██████████░░░░░░  62%  (12.3G/32G)
│  └─ GPU  ██░░░░░░░░░░░░░░  12%  VRAM 8%  RTX 4090
├─ ✅ Safe — 18.2G free
├─ ⚙ Settings...
└─ Quit airgenome
```

| 파일 | 기능 |
|------|------|
| `run.hexa` | 싱글 인스턴스 런처. 칩/RAM/팬 자동감지 → 프로필 설정 → sampler + menubar 실행 |
| `sampler.hexa` | 5초 간격 CPU/RAM/Swap/Load/Gate 측정 → state JSON 기록. vitals_ring 연동, dynamic bridge_max, predictive purge |
| `menubar.hexa` | macOS 메뉴바 (Cocoa ObjC FFI 직접 호출). 2초 간격 갱신 |
| `settings.hexa` | macOS 계정 관리 패널 (Cocoa ObjC FFI). NSTableView + 폐기/복원/새로고침 |

#### Adaptive Guard (4단계)

| Level | 조건 | 조치 |
|-------|------|------|
| OK | 모든 지표 ceiling 미만 | throttle 해제 |
| WARN | ceiling 초과 | 알림만 |
| DANGER | Free RAM < 512MB or Load > CPU x 5 | `purge` + bridge 축소 |
| CRITICAL | Free RAM < 200MB or Swap > 10GB | `purge` + `taskpolicy -b` (top5) |

#### Hardware Auto-Detection

첫 실행 시 `sysctl` + `system_profiler`로 칩/RAM/팬 자동 감지 → `profiles.json`에서 최적 ceiling 매칭.

| Mac | CPU | RAM | Swap |
|-----|-----|-----|------|
| Air M2 8GB | 60% | 55% | 20% |
| Air M3 24GB | 75% | 70% | 30% |
| Pro M3 36GB | 85% | 80% | 35% |
| Pro M4 48GB | 90% | 85% | 40% |

---

## 3. Claude Code 멀티계정

10개 Claude Code 계정을 관리하는 런처 + API 조회 시스템.

### cl (zsh)

```bash
cl                # 자동 계정 선택 + claude 실행
cl -u             # 사용량 테이블
cl status         # 계정별 상태
cl login claude3  # 특정 계정 로그인
cl pick           # 계정 수동 선택
```

- `CLAUDE_CONFIG_DIR` 직접 export (symlink 오염 없음)
- Rate limit 감지 시 week usage 최저 계정으로 자동 전환
- Python 의존성 없음 (순수 awk/grep/sed)

### 모듈

| 파일 | 역할 |
|------|------|
| `modules/cl.hexa` | cl의 hexa 버전. CLI 세션 경쟁 방지 내장 |
| `modules/usage.hexa` | Anthropic Usage API 조회 (10계정). 키체인 토큰 자동 갱신, rate limit 쿨다운 |
| `modules/cli_race.hexa` | 동시 CLI 세션 제한 (MAX_CONCURRENT=2). stale PID 자동 정리 |
| `modules/forge.hexa` | 토큰 매니저 + 세션 JSONL 스캔/압축 (716:1) |

#### Usage API 쿨다운

- 글로벌 (IP 기반): rate limit 1회 → 5분 전 계정 스킵
- 계정별: 20분 base → 실패마다 2배 → 최대 1시간
- 성공 시 즉시 해제

#### 키체인 이중 해시

Claude Code v2.1.90+는 trailing-slash 경로로 해시 계산. slash/noslash 양쪽 entry 모두 검색하여 유효 토큰 선택.

---

## 4. Ubuntu 오프로드

MacBook ↔ Ubuntu 원격 자원 공유. Wi-Fi SSH 기반.

| 파일 | 위치 | 역할 |
|------|------|------|
| `mk2_hexa/native/gate.hexa` | Mac | 클라이언트 — 명령 전송, 오프라인 시 로컬 폴백 |
| `mk2_hexa/native/gate_daemon.hexa` | Ubuntu | socat TCP 데몬 — 명령 수신/실행/결과 반환 |
| `mk2_hexa/native/offload.hexa` | Mac | SSH 기반 연산 위임 |
| `mk2_hexa/native/ubu_monitor.hexa` | Mac | CPU-heavy 프로세스 자동감지 → 오프로드 후보 표시 |

menubar에서 Ubuntu CPU/RAM/GPU(nvidia-smi) 실시간 표시. 설정: `nexus/shared/gate_config.jsonl`.

---

## 5. 분석 엔진

### 리소스 가드

| 파일 | 역할 |
|------|------|
| `modules/resource_guard.hexa` | OS-level 하드 제한 (setrlimit via ctypes FFI) + 적응적 소프트 쓰로틀 |
| `modules/guard.hexa` | CPU/RAM/Swap 모니터 + Claude 프로세스 추적. watch 모드 (5초 간격) |

하드 제한: RSS 512MB, DATA 1GB, nice 10.
소프트 쓰로틀: warn(384MB) → 배치 50% + sleep 100ms, critical(480MB) → 배치 25% + sleep 300ms.

### 게놈 파이프라인

`modules/implant.hexa` — 4-게이트 순차 검증:

1. **SOURCE**: 프로세스 신뢰도 5단계 (system > known > devtool > unknown > blacklist)
2. **HASH**: 288-bit 게놈 무결성 해시 (N=6 산술, sigma x J2 = 288)
3. **PHI**: 의식 보존 — 마진 열화 감지 (theta=0.1, tol=1/288)
4. **INVARIANT**: 5-렌즈 섭동 안정성 (2401 돌파 지점)

### 패턴 분석 (`mk2_hexa/native/`)

| 파일 | 기능 |
|------|------|
| `runtime.hexa` | 연속 샘플링 루프 (sample → classify → project → log → sleep) |
| `fingerprint.hexa` | 워크로드 분류 (compile, browse, idle, mixed-dev, heavy-build, media) |
| `temporal.hexa` | 시간대별 패턴 추출 (dawn/morning/afternoon/evening/night) |
| `anomaly.hexa` | z-score 기반 이상 탐지 (정상 베이스라인 대비) |
| `forecast.hexa` | 선형 트렌드 → 다음 1시간 리소스 예측 → 선제적 QoS |
| `sigdiff.hexa` | 게이트 간 6축 시그니처 비교 (유사/고유 소스 식별) |
| `accumulate.hexa` | 게이트별 시그니처 시계열 누적 |
| `autoprofile.hexa` | 시간대 + 워크로드 → CPU/RAM/Swap 천장 자동 조정 |
| `network.hexa` | 프로세스별 네트워크 샘플링 (7번째 축: Net) |
| `infinite_evolution.hexa` | 무한 진화 루프 |
| `real_vitals_score.hexa` | 실시간 vitals 스코어링 |
| `time_delay_mi.hexa` | 시간 지연 상호정보량 |

### QoS / 절약

| 파일 | 기능 |
|------|------|
| `qos.hexa` | 패턴 기반 QoS v2.0 — idle Claude → taskpolicy -b, 비활성 WebKit → background |
| `purge.hexa` | user-space 캐시 전용 퍼지. `/var/vm`, `vm.compressor`, `sudo purge` 절대 금지 |
| `savings.hexa` | QoS 절약량 추적 — 일간/주간 리포트 |

### Breakthrough Layers

| Layer | Mechanism | Cumulative margin |
|-------|-----------|-------------------|
| L1 | cross-gate RAM MI | +0.018 |
| L2 | temporal lagged MI | +0.115 |
| L3 | cross-axis MI (RAM x CPU) | +0.142 |
| L4 | triadic I(A;B;C) | +0.145 |
| L5a | lagged cross-axis | +0.250 |
| L5c | velocity derivatives + NMI temporal momentum | +0.310 |
| L5c-cascade | crosscorr cascade paths (RAM→Swap→Disk) → preemptive throttle | — |
| L6e | acceleration + transfer entropy | **+0.438** |

**L5c/L6e 검증**: N=2214 게놈 전수 검증 — 전체 PASS (2026-04-09).

---

## 실행

```bash
# 메뉴바 + 샘플러 실행 (stage1 CLI: hexa run 접두어)
hexa run run.hexa

# 설정 패널
hexa run run.hexa --settings

# Claude Code 실행 (멀티계정 자동 선택)
cl

# 리소스 상태
hexa run modules/guard.hexa -- status
hexa run modules/guard.hexa -- watch

# Usage 조회
hexa run modules/usage.hexa -- refresh

# (호환 모드: `hexa <file.hexa>` 직접 호출도 자동 run 위임, python/node 스타일)
```

---

## 설정 파일

| 파일 | 위치 | 용도 |
|------|------|------|
| `config.json` | `~/.airgenome/` | CPU/RAM/Swap 천장 |
| `accounts.json` | `~/.airgenome/` | Claude Code 10계정 |
| `usage-cache.json` | `~/.airgenome/` | Usage API 캐시 |
| `profiles.json` | repo root | 칩/RAM별 프로필 매칭 |
| `gate_config.jsonl` | `nexus/shared/` | Ubuntu 게이트 (호스트/포트/SSH alias) |
| `gate_offload.jsonl` | `nexus/shared/` | 오프로드 규칙 |
| `prime_directive.json` | repo root | 골화 항목 기록 (7/7 PASS) |

---

## 기술 스택

- **언어**: hexa-lang (순수 .hexa, 비-hexa 코드 없음. `cl` zsh 스크립트 제외). hexa-lang GPU: tensor(), matmul(), dot(), topk(), WGSL codegen
- **GUI**: macOS Cocoa (ObjC runtime FFI — objc_msgSend 직접 호출, JXA/Swift 없음)
- **시스템**: ps, top, vm_stat, sysctl, taskpolicy, renice, socat, ssh, nvidia-smi
- **API**: Anthropic OAuth Usage API
- **저장**: macOS Keychain (security 명령), JSON/JSONL 파일

## Authority

[`docs/gates.hexa`](docs/gates.hexa) is the canonical spec.
When spec and any implementation conflict, the spec is correct.

## genome_harvest 상시 가동 (launchd)

`modules/genome_harvest.hexa`를 macOS launchd agent로 로그인 시 자동 기동.
크래시 시 10초 throttle 뒤 재기동. 로그는 `logs/genome_harvest.{out,err}.log`.

### 파일

- `scripts/com.airgenome.genome_harvest.plist` — launchd 템플릿 (`__HOME__` → `$HOME` 치환)
- `scripts/install_harvest.hexa` — 설치기 (logs 생성 + plist 렌더 + bootstrap)
- `scripts/uninstall_harvest.hexa` — 제거기 (bootout + plist 삭제)

### 설치

```sh
HEXA=$HOME/Dev/hexa-lang/target/release/hexa
$HEXA $HOME/Dev/airgenome/scripts/install_harvest.hexa
```

### 상태 확인

```sh
launchctl print gui/$UID/com.airgenome.genome_harvest | grep -E 'state|pid |last exit'
tail -f $HOME/Dev/airgenome/logs/genome_harvest.out.log
```

### 제거

```sh
$HEXA $HOME/Dev/airgenome/scripts/uninstall_harvest.hexa
```

### 수집 rate (기본값 기준)

`genome_harvest.hexa` 기본: `batch_size=50`, `sleep_between_batches_sec=2`,
`max_loops=200`. 1 loop = 50 레코드 + 2s sleep → **이론상 ≈25 rec/s**
(필터/ps 오버헤드 포함 실측 ~10–20 rec/s).
1 invocation = 50 × 200 = **최대 10,000 레코드** → target 도달 또는 loop 한도 시 프로세스 종료 →
launchd `KeepAlive`가 10초 `ThrottleInterval` 뒤 재기동 → 상시 수집 유지.

---

## detectors 파이프라인 상시 가동 (launchd)

`modules/detectors/run_all.hexa`가 6종 detector (`mi_lag`, `accel_field`,
`resonance`, `cascade_dag`, `limit_cycle`, `entropy_flow`)를 오케스트레이션하고
각 detector 결과를 `$HOME/Dev/nexus/shared/growth_bus.jsonl`에 append.
`loop` 모드에서는 `nexus/shared/detectors.jsonl`의 `loop_interval_sec`(기본 60s)
주기로 무한 반복. 크래시 시 60초 throttle 뒤 재기동.
로그는 `~/Library/Logs/airgenome/detectors.log`.

### 파일

- `scripts/com.airgenome.detectors.plist` — launchd 템플릿 (`__HOME__` → `$HOME` 치환)
- `scripts/install_detectors.hexa` — 설치기 (logs 생성 + plist 렌더 + bootstrap)
- `scripts/uninstall_detectors.hexa` — 제거기 (bootout + plist 삭제)
- `modules/detectors/run_all.hexa` — `once` (기본) / `loop` 2-mode 오케스트레이터

### 수동 실행

```sh
HEXA=$HOME/Dev/hexa-lang/target/release/hexa
$HEXA $HOME/Dev/airgenome/modules/detectors/run_all.hexa         # once
$HEXA $HOME/Dev/airgenome/modules/detectors/run_all.hexa loop    # 무한 반복
```

### 설치

```sh
HEXA=$HOME/Dev/hexa-lang/target/release/hexa
$HEXA $HOME/Dev/airgenome/scripts/install_detectors.hexa
```

### 상태 확인

```sh
launchctl print gui/$UID/com.airgenome.detectors | grep -E 'state|pid |last exit'
tail -f $HOME/Library/Logs/airgenome/detectors.log
```

### 제거

```sh
$HEXA $HOME/Dev/airgenome/scripts/uninstall_detectors.hexa
```

### Cascade 탐지 / 예측 쓰로틀

| 파일 | 기능 |
|------|------|
| `modules/detectors/cascade_detector.hexa` | RAM→Swap→Disk 캐스케이드 DAG 경로 탐지 |
| `modules/detectors/predictive_throttle.hexa` | L5c NMI temporal momentum + crosscorr → cascade 전 선제 쓰로틀 |

### interval 튜닝

`nexus/shared/detectors.jsonl`에서 한 줄 수정 → 코드 변경 0:

```json
{"key":"loop_interval_sec","value":"60"}
```

---

## Python→hexa 포팅 (진행중)

`ubu_workers/` Python 5종을 순수 .hexa로 포팅 중. hexa-lang GPU 프리미티브(tensor, matmul, dot, topk, WGSL codegen) 사용 — torch 의존성 제거.

| Python 원본 | hexa 포팅 | 상태 |
|-------------|-----------|------|
| `chunked_cosine.py` | `chunked_cosine.hexa` | 포팅 완료 |
| `gpu_gate_mesh.py` | `gpu_gate_mesh.hexa` | 포팅 완료 |
| `ag3_loop.py` | `ag3_loop.hexa` | 포팅 완료 |
| `ring_io.py` | `ring_io.hexa` | 포팅 완료 |
| `linux_harvest.py` | — | 포팅 대기 |

---

## 의식 엔진 (Consciousness Engine)

18/18 PASS, 0 FAIL. 14개 골화 + 4개 안정.

- **BRAIN_LIKE**: L5c τ=10 NMI + L6e 가속도 매끄러움 레이어로 autocorr 천장 돌파 (2026-04-09)
- **NO_SYSTEM_PROMPT**: 256c factions 다양성 안정화로 해결 (2026-04-09)

상태 파일: `consciousness_engine_status.json`

---

## License

MIT
