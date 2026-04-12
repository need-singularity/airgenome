# airgenome Filter Architecture — 데이터 재해석 필터 정의

**Date**: 2026-04-12
**Status**: L0 골화

## 필터란

필터는 **raw 데이터를 재해석하여 숨겨진 의미를 추출하고, 그 의미를 시스템에 전달하여 성능과 자원을 개선하는 변환 계층**이다.

필터는 데이터를 삭제하거나 차단하는 것이 아니다. 데이터의 **의미를 변환**하는 것이다.

```
raw data → [FILTER: 재해석] → interpreted signal → [시스템 개선]
```

## 핵심 원리

### 1. 데이터 재해석 (Reinterpretation)

동일한 raw 데이터도 맥락에 따라 다른 의미를 가진다:

| raw data | 맥락 없음 | 필터 재해석 |
|----------|-----------|-------------|
| `ubu load=0.5` | "부하 낮음" | "12코어 중 95% 유휴 → 자원 낭비 → 작업 투입 필요" |
| `htz ram=99%` | "메모리 많이 씀" | "OOM 임계점 → 신규 작업 라우팅 금지 + 캐시 정리" |
| `Safari WebContent CPU 0.1%` | "프로세스 존재" | "background 탭 → BG policy 적용 가능 → Mac 자원 회수" |
| `ubu GPU 0%` | "GPU 안 씀" | "RTX 5070 유휴 → GPU 작업 미디스패치 → 파이프라인 병목" |
| `ssh 13개 동시접속` | "접속 많음" | "sshd MaxStartups 초과 임박 → 세마포어 필요" |

재해석은 raw value를 **시스템 상태 + 물리적 한계 + 정책** 맥락에서 읽는 것이다.

### 2. 양자얽힘 (Entanglement)

호스트 간 상태는 독립이 아니라 **얽혀있다**:

```
ubu CPU 5%  ←→  htz CPU 50%  ←→  Mac load 10
     ↕               ↕               ↕
  GPU 0%         RAM 99%         오케스트레이션
```

하나의 상태 변화가 전체 시스템의 최적 상태를 바꾼다:
- ubu가 유휴 → htz에서 ubu로 작업 이동이 최적
- htz RAM 포화 → ubu가 유일한 compute 호스트
- ubu SSH 불통 → htz만 사용 가능 → htz 과부하 위험

**얽힘 감지**: 두 호스트의 상태를 동시에 읽고, 상관관계에서 액션을 도출하는 것이 필터의 역할.

### 3. 전달 (Propagation)

재해석된 시그널은 시스템의 다음 계층으로 **전달**되어 개선을 일으킨다:

```
[infra_probe] → raw metrics
       ↓
[load_balancer] → 필터: 호스트 스코어링 (cpu*w + ram*w + gpu*w)
       ↓
[dispatch_state.json] → 시그널: best host per task type
       ↓
[auto_dispatch] → 필터: Mac hog 감지 + 오프로드 결정
       ↓
[ssh_gate] → 필터: 세마포어 + 메트릭 피기백 + 재해석
       ↓
[원격 실행] → 실제 자원 개선 (작업 투입 / renice / cache flush)
```

매 계층이 이전 계층의 출력을 **재해석**하고, 다음 계층에 **더 높은 수준의 시그널**을 전달한다.

## 필터 유형

### Type A: 프로세스 게이트 필터

Mac 프로세스를 관찰하고 자원 회수 추천을 생성한다.

| 게이트 | raw data | 재해석 | 출력 |
|--------|----------|--------|------|
| safari_gate | WebContent PID, CPU% | "background 탭 vs foreground 탭" | `taskpolicy_bg` 추천 |
| telegram_gate | Telegram PID, 통화 상태 | "통화 중이면 절대 건들지 마" | 조건부 `taskpolicy_bg` |
| claude_gate | Claude PID, cwd, idle | "같은 cwd 중복 + idle → 낭비" | `renice` 추천 |
| memo_gate | Notes PID, frontmost | "background Notes → 자원 회수" | `taskpolicy_bg` 추천 |
| finder_gate | Finder helper PID | "helper만 대상, 본체 절대 금지" | `taskpolicy_bg` 추천 |

**패턴**: `ps 관찰 → 맥락 재해석 → JSONL 추천 → coordinator 적용`

### Type B: 호스트 밸런싱 필터

호스트 간 자원 상태를 관찰하고 작업 라우팅을 결정한다.

| 필터 | raw data | 재해석 | 출력 |
|------|----------|--------|------|
| load_balancer | infra_state.json | "host별 free% → task type별 최적 호스트" | dispatch_state.json |
| resource_ceiling | infra_state.json | "총 활용도 vs 천장 → gap%" | auto-fill 트리거 |
| auto_dispatch | Mac ps + dispatch_state | "Mac CPU hog + ubu 여유 → 오프로드" | SSH 작업 투입 |

**패턴**: `infra_state 읽기 → 스코어링/얽힘 분석 → dispatch_state 쓰기 → 실행`

### Type C: 트랜스포트 필터 (SSH Gate)

SSH 트래픽 자체를 데이터 소스로 활용한다.

| 필터 계층 | raw data | 재해석 | 출력 |
|-----------|----------|--------|------|
| 명령 필터 | SSH 명령 문자열 | "위험 vs 안전 패턴 매칭" | DENY / WARN / ALLOW |
| 세마포어 | 동시 접속 수 | "slotN/maxN → 포화도" | BLOCKED / PASS |
| 메트릭 피기백 | /proc/loadavg + meminfo | "매 호출 = 무료 프로브" | metrics.json |
| 재해석 | 수집된 메트릭 | "cpu_idle / ram_critical / overload" | signals.json + 자동 액션 |

**패턴**: `SSH 호출 → 4층 필터 스택 → 시그널 + 자동 개선`

### Type D: 게놈 필터 (Forge)

대화/세션 데이터를 6축 게놈으로 투영한다.

| 필터 | raw data | 재해석 | 출력 |
|------|----------|--------|------|
| genome projection | JSONL messages | "token throughput → CPU축, tool density → GPU축" | 60-byte genome |
| signature diff | 게놈 시계열 | "시그니처 변화 패턴 → 세션 특성 진화" | per-source diff |

**패턴**: `raw JSONL → 6축 투영 → 게놈 시그니처 → 패턴 분석`

## 필터 설계 원칙

### P1: 관찰은 무료여야 한다
필터는 기존 데이터 흐름에 **기생**(piggyback)한다. 별도 프로브/폴링 금지.
- SSH 게이트: 이미 실행하는 SSH에 `/proc` 읽기 피기백 (~1ms)
- 프로세스 게이트: 이미 도는 coordinator가 `ps` 1회 호출, 7개 게이트가 공유
- 게놈: 이미 존재하는 JSONL을 읽음

### P2: 재해석은 맥락 의존
같은 `load=10`도:
- 12코어에서 = 83% (위험)
- 32코어에서 = 31% (여유)
- Mac에서 = AG6 위반 (compute 금지)

필터는 반드시 **물리적 한계 + 정책**을 알아야 한다.

### P3: 출력은 시그널이다
필터 출력은 직접 실행이 아니라 **시그널/추천**이다:
- `{"signal": "cpu_idle", "action": "fill_cpu"}` → auto_dispatch가 실행
- `{"app": "safari", "action": "taskpolicy_bg"}` → coordinator가 실행

분리 이유: 필터는 판단, 실행은 집행기. 집행기에 안전장치(HARD_NEVER, 5중 가드).

### P4: 얽힘은 cross-read
단일 호스트 데이터만으로는 최적화 불가. 필터는 **여러 호스트/프로세스의 상태를 동시에** 읽어서 상관관계를 추출한다:
- load_balancer: ubu + htz 동시 스코어링
- auto_dispatch: Mac CPU + ubu 여유 + htz 여유 삼각 분석
- ssh_gate: 호스트별 메트릭 + 시그널 교차 참조

### P5: 필터는 겹쳐 쌓인다
단일 필터가 아니라 **필터 스택**이 데이터를 점진적으로 정제한다:

```
Layer 0: raw (ps, /proc, JSONL)
Layer 1: 정규화 (load→%, ram→%, PID→역할)
Layer 2: 맥락 부여 (frontmost, 정책, 물리한계)
Layer 3: 얽힘 분석 (호스트 간 상관, 시계열 패턴)
Layer 4: 시그널 생성 (fill/drain/renice/block)
Layer 5: 전달 → 집행기
```

## Type E: 데이터 재해석 필터 (원형)

기존 구현체 — 필터 개념의 **원형이자 본질**. OS 자원 관리가 아니라 **데이터 자체의 숨겨진 구조를 발견하여 성능을 개선**하는 것.

### E1: 양자 얽힘 필터 (`claude_quantum_filter.hexa`)

```
raw JSONL → entanglement 탐지 → 얽힌 key pair drop → lossless 복원 가능
```

**원리**: JSONL의 수백 개 key 중 **항상 같은 값으로 함께 움직이는 pair** (양자 얽힘)를 발견. 하나만 남기고 나머지는 제거. 복원 시 헤더의 매핑 테이블로 원복.

- 입력: Claude conversation `.jsonl` (수 GB)
- 재해석: key pair 상관 분석 → entanglement map
- 출력: `.qjsonl.gz` (lossless, 원본 대비 대폭 축소)

### E2: 바이트 재해석 필터 (`claude_byte_reinterpret.hexa`)

```
raw JSONL → SESSION-CONSTANT 추출 → 반복 제거 → 재인코딩
```

**원리**: gzip 같은 범용 압축이 아니라, **JSONL의 의미 구조를 이해**해서 같은 정보를 적은 바이트로 재인코딩. `sessionId`, `cwd`, `version` 등 모든 줄에 반복되는 필드를 헤더 1회로 추출.

### E3: 런타임 가속 필터 (`claude_runtime_filter.hexa`)

```
cold JSONL → entanglement-collapsed → msgpack blob → in-memory 즉시 반복
```

**원리**: 디스크의 JSONL을 읽을 때마다 파싱하지 않고, **재인코딩된 바이너리 blob**을 `/tmp`에 캐시. hot path에서 JSON parse 0회.

### E4: Safari mmap 필터 (`safari_runtime_filter.hexa`)

```
History.db → mmap binary blob (SHBF) → bisect 즉답
```

**원리**: SQLite B-tree 탐색 대신, **정렬된 바이너리 blob**을 mmap하여 binary search. autocomplete/history 검색이 sqlite open 없이 즉답.

### E5: SQLite 재해석 (`sqlite_byte_reinterpret.hexa`)

```
live DB → online backup → VACUUM 시뮬 → page 재배치 → byte 회수 측정
```

**원리**: 앱이 사용 중인 DB를 안전하게 복사 → 프래그먼트 분석 → 실제 절약량 측정. `--apply`로 교체 가능.

### E6: 코덱 벤치마크 (`quantum_byte_bench.hexa`)

모든 재해석 기법을 정량 비교:
- 양자 얽힘 (entanglement drop)
- 텔레포트 (원격 상태 전달)
- 홀로그래픽 (경계 데이터에서 전체 복원)
- 엔트로피 분리 (고엔트로피/저엔트로피 스트림 분리 압축)

**이들이 필터의 본질**: raw data의 **반복, 상관, 구조**를 발견하고, 그것을 제거/변환하여 같은 정보를 적은 자원으로 표현하는 것.

## 현재 필터 맵

```
                    ┌─ safari_gate ─┐
                    ├─ telegram_gate─┤
infra_probe ──→ infra_state ──→ load_balancer ──→ dispatch_state
    │                │                                   │
    │          resource_ceiling                    auto_dispatch
    │                │                                   │
    └──── ssh_gate ──┴── 메트릭 피기백 ──→ signals.json  │
              │                                          │
         [DENY/WARN]                              [SSH 작업 투입]
         [세마포어]                                [오프로드]
         [재해석]
```

모든 화살표가 **필터**다. 데이터가 흐르면서 의미가 정제되고, 최종적으로 시스템이 개선된다.
