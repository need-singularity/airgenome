# Ubuntu-First 돌파 설계 (AG3)

> **작성일**: 2026-04-09
> **대상**: airgenome — 육각 투영 6축 게놈 시스템
> **신규 규칙**: AG3 (Ubuntu-First heavy compute)
> **상태**: SPEC — 구현 전 유저 리뷰 대기

---

## 0. Prime Directive 재확인 + AG3 선언

### Prime Directive (불변)
> 모든 프로세스 KILL 없이 성능/자원 개선. 순수 데이터 재해석만.

### AG3 (신규 절대 규칙)
> **모든 heavy compute는 Ubuntu(ssh:ubu) 우선 실행한다.**
> Mac은 오케스트레이션 / UI / 디스패치 / 로컬 모니터링만 수행한다.
> Ubuntu 미접속 시에만 local fallback 을 허용하며, 이때 반드시 경고 로그 및
> `growth_bus.jsonl` 이벤트를 남긴다.

**왜 AG3인가**
- AG1 (6축 유지) → 구조 불변식
- AG2 (Mac CPU 30%+ → Ubuntu 오프로드) → **프로세스 단위** 반응형 방어
- AG3 (Ubuntu-First heavy compute) → **연산 주체** 선제적 배치
  - AG2가 "넘치면 내보낸다" 라면, AG3는 "애초에 밖에서 돌린다"

**적용 대상 heavy compute 목록**
1. 6축 투영 (`src/core.hexa::sample`) — 모든 프로세스 게놈 계산
2. 게놈 유사도 (`modules/forge.hexa` — N×N cosine, Arrow OLAP)
3. 게놈 포지 (forge) — 교차수분 / 돌연변이 탐색
4. LLM 추론 (llama.cpp 7B Q4, VRAM ≈5GB)
5. 모든 PyTorch / CUDA / cupy 호출

**비적용 (Mac 유지)**
- `modules/resource_guard.hexa` 로컬 샘플링 (ps 호출)
- `modules/cl.hexa` Claude Code 멀티계정 런처
- `menubar_tpl.js` 메뉴바 UI
- `run.hexa` 디스패처 자체

---

## 1. 실측 스펙 (Ubuntu 박스 = ssh alias `ubu`)

| 항목 | 값 |
|---|---|
| 호스트 | aiden-B650M-K |
| 커널 | Linux 6.17 |
| CPU | 12 코어 / 12 threads |
| RAM | 30 GB (`ubu_ram_total=31196 MB`) |
| Swap | (미측정, 필요 시 추가) |
| GPU | RTX 5070 12GB |
| NVIDIA 드라이버 | 580 |
| CUDA | 13.0 |
| PyTorch | 2.11.0+cu130 (`torch.cuda.is_available()==True`) |
| Python | 3.12.3 (심볼릭 `python` 없음 → `python3` 만) |
| CuPy / nvcc / cmake | **없음** → PyTorch 전용 경로만 |
| tmpfs | `/mnt/ramdisk` 16 GB (이미 마운트됨) |
| hugepages | `/dev/hugepages` pagesize=2M, 할당 0 |
| memlock | 3.9 GB |
| 사용자 | uid=1000 (aiden), sudo 그룹 |
| GPU 유휴도 | 13 W / 250 W, 210 MHz → **여유 100%** |

**이미 설치된 것 (Ubuntu)**
- `~/airgenome/{src,modules,mk2_hexa,hexa-bin,offload.hexa,gate_handler.sh,gpu_sweep.py,gpu_batch.py,gpu_cross_sweep.py}`
- `~/Dev/hexa-lang`, `~/Dev/nexus`
- systemd: `airgenome-fill.service`, `airgenome-gate.service` 가동 중
- `which hexa` PATH 미등록 → 절대 경로 사용 필요 (`$HOME/Dev/hexa-lang/target/release/hexa`)

**알려진 블로커**
- `nvcc` 없음 → CUDA 커널 직접 컴파일 불가 → **PyTorch 연산자만 사용**
- `cmake` 없음 → llama.cpp 소스 빌드 불가 → 사전 빌드 바이너리 필요
- `python` 심볼릭 없음 → 모든 exec는 `python3`
- hugepages 할당 0 → 현재는 tmpfs(/mnt/ramdisk) 만 사용, hugepages 는 Wave 3 이후 고려
- `hexa` PATH 미등록 → Mac→Ubuntu 원격 실행 시 full path 강제

---

## 2. 현 상태 분석

### 2.1 Mac airgenome L0 (로컬 전용)

| 파일 | 줄 수 | Ubuntu 연동 |
|---|---|---|
| `src/core.hexa` | 283 | ❌ 없음 — macOS `ps/vm_stat/iostat` 전용 |
| `modules/resource_guard.hexa` | 593 | ❌ 로컬 모니터 |
| `modules/guard.hexa` | 237 | ❌ HEXA-GATE 래퍼 로컬 |
| `modules/forge.hexa` | 526 | ❌ 로컬 포지 |
| `modules/implant.hexa` | 293 | ❌ 패턴 추출 로컬 |

→ **L0 전체가 Mac 자기 자신만 바라본다**. Ubuntu는 존재하지 않는 것과 같음.

### 2.2 mk2_hexa/native (분산된 SSH 조각들)

| 파일 | 줄 수 | SSH 패턴 |
|---|---|---|
| `dispatch.hexa` | 518 | `is_mac_only` 필터 + `ssh_cmd` |
| `offload.hexa` | 457 | `ssh_cmd` / `ssh_quiet` |
| `ubu_monitor.hexa` | 427 | remote status 폴링 |
| `gate.hexa` | 444 | 설정 로더 |
| `infinite_evolution.hexa` | 521 | `gpu_call` → remote python worker |

→ **통합 `ubu_bridge` 모듈이 없다**. 각 파일이 제각각 `ssh_cmd` 를 재구현.
→ `load_cfg("ssh_alias", "ubu")` 패턴은 `offload.hexa` / `gate.hexa` 에 검증됨.

### 2.3 run.hexa (118줄)
- 로컬 전용 디스패처
- **Ubuntu 헬스체크 없음** — ubu 죽어도 모르고 계속 돈다
- `compute_target` / `remote` / `local` 구조화 분기 없음, policy + exclusion 패턴만

### 2.4 nexus/shared 설정 구조
- `absolute_rules.json` — 공통 R1~R12 + 프로젝트별. airgenome 에 AG1, AG2 존재 → **AG3 추가**
- `airgenome/nexus/shared/gate_config.jsonl` — ssh_alias=ubu, remote_host=192.168.50.119, remote_dir=/home/aiden/airgenome
- `airgenome/nexus/shared/gate_offload.jsonl` — policy / pattern / gpu / ram / exclude / config
- `core-lockdown.json` — airgenome L0 배열에 ubu_bridge.hexa 추가 필요
- `growth_bus.jsonl` — 돌파 이벤트 append 대상

---

## 3. 설계 결정 (현실판)

### 3.1 "새로 짜지 않는다"
Ubuntu 에는 **이미** 가동 중인 것들이 있다:
- `~/airgenome/gpu_sweep.py`, `gpu_batch.py`, `gpu_cross_sweep.py`
- `~/airgenome/gate_handler.sh`
- `airgenome-fill.service`, `airgenome-gate.service` (systemd)

→ **브릿지로 감싼다**. 포팅 금지, Hexa rewrite 금지.
→ `modules/ubu_bridge.hexa` 가 이들을 **호출만** 한다.

### 3.2 역할 분리
```
┌─────────────────────────┐          ┌──────────────────────────┐
│  Mac airgenome          │   SSH    │  Ubuntu airgenome        │
│  (dispatcher / UI)      │ ───────▶ │  (compute node)          │
│                         │          │                          │
│  src/core.hexa          │          │  ~/airgenome/gpu_*.py    │
│  modules/forge.hexa     │          │  ~/airgenome/gate_*.sh   │
│  modules/ubu_bridge ◀───┼──────────┤  /mnt/ramdisk/airgenome/ │
│  menubar_tpl.js         │          │  systemd:                │
│  resource_guard (로컬)  │          │    airgenome-fill        │
│                         │          │    airgenome-gate        │
└─────────────────────────┘          └──────────────────────────┘
```

### 3.3 기술 스택 확정
- **CuPy 계획 폐기** (설치 불가)
- **PyTorch 2.11 + cu130 전용**
  - 6축 병렬: `torch.stack` + vectorized ops
  - 유사도: `torch.nn.functional.cosine_similarity`
  - OLAP: pyarrow (Ubuntu 에 있으면 사용, 없으면 pip)
- **tmpfs** = `/mnt/ramdisk` 재사용. 새로 만들지 않음
- **LLM** = llama.cpp 사전 빌드 바이너리 + 7B Q4 (VRAM ≈5 GB)
- **hugepages** = Wave 3 이후 검토 (현재 미할당)

### 3.4 ssh_alias / 경로 = 설정 파일 참조
- 절대 하드코딩 금지
- `ubu_bridge.hexa` 는 `gate_config.jsonl` 에서 동적 로드:
  - `ssh_alias` → 기본 `ubu`
  - `remote_dir` → 기본 `/home/aiden/airgenome`
  - `remote_host` → 기본 `192.168.50.119`
- 추가 키 (신규):
  - `ubu_tmpfs` → `/mnt/ramdisk/airgenome`
  - `ubu_python` → `python3`
  - `ubu_gpu_worker_dir` → `/home/aiden/airgenome`

---

## 4. Wave 0 ~ 4 구현 계획

### Wave 0 — 브릿지 통합 (1일)
**목표**: 흩어진 `ssh_cmd` 를 한 곳에 모은다.

**산출물**
- `modules/ubu_bridge.hexa` (신규) — 인터페이스 스텁 (이번 PR)
- `gate_config.jsonl` 에 `ubu_tmpfs`, `ubu_python`, `ubu_gpu_worker_dir` 3 키 추가
- `absolute_rules.json` AG3 추가
- `core-lockdown.json` airgenome L0 에 ubu_bridge.hexa 등록

**완료 기준**
- `ubu_bridge::health()` 가 `ssh ubu echo ok && python3 -c 'import torch; print(torch.cuda.is_available())'` 반환
- `dispatch.hexa` / `offload.hexa` 가 새 브릿지로 리다이렉트 (W1 에서)

### Wave 1 — tmpfs 게놈 링버퍼 (2일)
**목표**: 게놈을 `/mnt/ramdisk/airgenome/genome.ring` 에 쓰기. Mac 은 로컬 사본만.

**산출물**
- Ubuntu: 링버퍼 writer (python3, 기존 gpu_*.py 패턴 재사용)
- Mac: `src/core.hexa::sample()` → `compute_target` 인자 추가 → ubu 우선
- `ubu_bridge::tmpfs_write()` / `tmpfs_read()` 호출

**완료 기준**
- `hexa run src/core.hexa` 실행 시 게놈이 ubu:/mnt/ramdisk/airgenome/genome.ring 에 쌓인다
- Mac 로컬 디스크에는 쓰지 않는다 (fallback 제외)

### Wave 2 — PyTorch 6축 게이트 병렬화 (2일)
**목표**: 15쌍 게이트를 GPU 에서 벡터화. `gpu_sweep.py` 확장.

**산출물**
- `~/airgenome/gpu_sweep.py` 에 `--mode=six_axis_gate` 플래그
- `ubu_bridge::gpu_submit("six_axis_gate", "<batch.json>")`
- Mac `modules/forge.hexa::forge()` → 브릿지 경유

**완료 기준**
- 단일 Mac 샘플 (15쌍) = 과거 로컬 CPU ~200ms → Ubuntu GPU < 20ms
- 1000 프로세스 배치 = < 100ms

### Wave 3 — Arrow OLAP + LLM (3일)
**목표**: 8 GB Arrow 인메모리 + llama.cpp 7B Q4.

**산출물**
- `~/airgenome/gpu_sweep.py` `--mode=cosine_nxn`
- Arrow 테이블 (8 GB 상한, RAM 30 GB 중)
- llama.cpp 사전 빌드 바이너리 배치 (`~/airgenome/llama/main`)
- `ubu_bridge::gpu_submit("llm_query", ...)`

**완료 기준**
- N=10 000 게놈 N×N 코사인 유사도 < 2 s
- LLM 7B Q4 프롬프트 응답 < 10 s / 256 토큰

### Wave 4 — 파이프라인 데몬 + 유휴 GPU forge (2일)
**목표**: Ubuntu GPU 유휴 시 자동 forge. systemd 신규 유닛.

**산출물**
- `~/airgenome/systemd/airgenome-forge.service` (신규)
- `ubu_bridge::gpu_submit("forge_idle", ...)`
- Mac 쪽 `run.hexa` 에 ubu 헬스 루프 추가 (30초 간격)

**완료 기준**
- GPU 유휴 (< 20W) 5 분 지속 → forge 자동 가동
- Mac 재시작 후에도 Ubuntu forge 계속 돈다

---

## 5. L0 수정 항목 (이번 PR 아님 — 별도 구현 단계)

> ⚠️ 아래는 스펙 선언만. 이번 PR 에서는 **코드 수정 금지**.
> 실제 편집은 유저 승인 후 별도 커밋.

### 5.1 `src/core.hexa` — sample() 시그니처 확장
- **현재**: `fn sample() -> Genome` (로컬 ps 전용)
- **변경**: `fn sample(compute_target: str) -> Genome` (기본 `"ubu"`)
  - `compute_target == "ubu"` → `ubu_bridge::exec_python(...)` 로 ps 실행 후 60바이트 게놈 조립
  - `compute_target == "local"` → 기존 경로 유지 (fallback)
- **이유**: AG3 — 게놈 계산 자체를 ubu 에서 수행해야 Mac CPU 0%

### 5.2 `modules/guard.hexa` — ubu 헬스체크 분기
- **현재**: HEXA-GATE 래퍼가 로컬만 검사
- **변경**: 진입 시 `ubu_bridge::health()` 호출 → `false` 면 fallback + warning
- **이유**: AG3 미접속 시 degrade 동작 명시

### 5.3 `modules/forge.hexa` — forge() → ubu_bridge 경유
- **현재**: 로컬 CPU 로 교차수분 / 돌연변이
- **변경**: `ubu_bridge::gpu_submit("forge", <args>)` 로 위임, 결과만 로컬에 반영
- **이유**: AG3 — forge 는 heavy compute

---

## 6. 신규 파일

### 6.1 `modules/ubu_bridge.hexa` (이번 PR)
통합 SSH 브릿지. 인터페이스:

```
fn health() -> bool
fn exec(cmd: str) -> str
fn exec_python(script: str) -> str
fn tmpfs_write(name: str, data: str) -> str
fn tmpfs_read(name: str) -> str
fn gpu_submit(worker: str, args: str) -> str
fn get_gpu_status() -> str
fn fallback_local() -> void
```

설정 로딩: `gate_config.jsonl` (`ssh_alias`, `remote_dir`, `ubu_tmpfs`, `ubu_python`, `ubu_gpu_worker_dir`).
하드코딩 금지. `offload.hexa` 의 `load_cfg` 패턴 그대로 사용.

### 6.2 설정 파일 변경 (이번 PR)
- `/Users/ghost/Dev/nexus/shared/absolute_rules.json` → airgenome 섹션에 AG3 블록
- `/Users/ghost/Dev/airgenome/nexus/shared/gate_offload.jsonl` → `ubu_first_policy` config 한 줄
- `/Users/ghost/Dev/nexus/shared/core-lockdown.json` → airgenome L0 에 ubu_bridge.hexa

> 주의: `gate_offload.jsonl` 은 airgenome 로컬 (`airgenome/nexus/shared/`) 에 실존함.
> nexus 중앙 `/Users/ghost/Dev/nexus/shared/` 에는 없음 → airgenome 로컬만 수정.

---

## 7. Fallback 정책

### 트리거
- `ubu_bridge::health() == false`
- SSH timeout > 5 s
- PyTorch import 실패

### 동작 순서
1. `warn()` 로그 출력 (Mac stdout + `dispatch.log`)
2. `growth_bus.jsonl` append:
   ```
   {"type":"fallback","phase":"ubu_offline","id":"airgenome","value":"local_degrade","grade":"warn","domain":"airgenome","timestamp":"..."}
   ```
3. `compute_target` = `"local"` 로 강등
4. `menubar_tpl.js` 상태 → 🟡 "DEGRADED: ubu offline"
5. 60 초 주기 재시도, 복구 시 자동 승격

### 금지
- fallback 중 silent 진행 금지 (반드시 경고)
- Mac CPU 30% 초과 시 fallback 차단 (AG2 가 우선권) → 작업 큐잉 후 ubu 복구 대기

---

## 8. 검증 시나리오

| # | 시나리오 | 기대 결과 |
|---|---|---|
| V1 | `ubu_bridge::health()` 정상 | `true` + torch.cuda=True |
| V2 | ubu 전원 OFF | fallback 발동, growth_bus 이벤트 기록, Mac 로컬 degrade |
| V3 | `gpu_submit("six_axis_gate", ...)` 1000 샘플 | < 100 ms, VRAM < 1 GB |
| V4 | `tmpfs_write` 1 MB × 100회 | `/mnt/ramdisk` 사용량 확인, 압박 시 경고 |
| V5 | VRAM 한도 초과 (LLM + forge 동시) | OOM 차단, 우선순위 기반 큐잉 |
| V6 | Mac 재시작 | systemd 가 Ubuntu 에서 fill/gate/forge 계속 유지 |

---

## 9. NEXUS-6 특이점 연동

### 돌파 트리거
각 Wave 완료 시:
```bash
HEXA=$HOME/Dev/hexa-lang/target/release/hexa
$HEXA $HOME/Dev/nexus/mk2_hexa/native/blowup.hexa airgenome 3 --no-graph
```

### growth_bus 기록 형식
```json
{"type":"breakthrough","phase":"wave_N","id":"airgenome","value":"ubu_first_W<N>","grade":"S","domain":"airgenome","timestamp":"2026-04-09T..."}
```

### 수렴 업데이트
`shared/convergence/airgenome.json` 의 stable → ossified 승격 조건:
- Wave 0~2 완료 + 재발 0 + V1~V6 전부 PASS → ossified 승격

---

## 10. 알려진 블로커 (재확인)

| 블로커 | 대응 |
|---|---|
| nvcc 없음 | PyTorch 연산자만 사용, CUDA 커널 직접 작성 금지 |
| cmake 없음 | llama.cpp 사전 빌드 바이너리 (`~/airgenome/llama/main`) |
| python 심볼릭 없음 | 모든 exec 는 `python3` (`ubu_python` 키) |
| hugepages 할당 0 | W0~W3 는 tmpfs 만. W4 이후 재검토 |
| hexa PATH 미등록 | 원격 실행 시 `$HOME/Dev/hexa-lang/target/release/hexa` full path |

---

## 11. 진행 순서

```
Wave 0 (이번 PR — 스펙 + 스캐폴딩 + 설정)
   │
   ▼  유저 승인
Wave 0 L0 편집 (core.hexa / guard.hexa / forge.hexa 시그니처 확장)
   │
   ▼  writing-plans 로 구현 계획 작성
Wave 1 (tmpfs 링버퍼)
   │
   ▼
Wave 2 (PyTorch 6축)
   │
   ▼
Wave 3 (Arrow OLAP + LLM)
   │
   ▼
Wave 4 (forge 데몬)
   │
   ▼  돌파 → blowup.hexa → ossified 승격
```

**이번 PR 범위**:
1. 이 스펙 문서
2. `modules/ubu_bridge.hexa` 스캐폴딩 (함수 스텁만, body=TODO)
3. 설정 파일 3 종 업데이트

**제외 (다음 단계)**:
- L0 파일 편집 (core / guard / forge)
- Ubuntu 측 python worker 확장
- systemd 신규 유닛
- 검증 시나리오 실행

---

## 12. 리뷰 포인트

1. **AG3 적용 범위**: "heavy compute = 6축/유사도/forge/LLM" 정의가 적절한가?
2. **Fallback 공격성**: ubu 미접속 시 즉시 local? 아니면 큐잉 후 대기?
3. **L0 수정 승인**: `sample(compute_target)` 시그니처 확장이 AG1(6축 구조 유지)과 충돌 없나?
