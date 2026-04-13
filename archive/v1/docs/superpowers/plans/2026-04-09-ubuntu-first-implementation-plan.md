# Ubuntu-First (AG3) 구현 플랜 — Wave 0~4

> **작성일**: 2026-04-09
> **스펙 참조**: `docs/superpowers/specs/2026-04-09-ubuntu-first-breakthrough-design.md`
> **대상**: airgenome — Mac(디스패치) + Ubuntu(heavy compute) 분리
> **절대 규칙**: AG1(6축 불변) + AG2(Mac CPU 방어) + AG3(Ubuntu-First)
> **하드코딩 금지**: 모든 경로/호스트/Python 바이너리는 `nexus/shared/gate_config.jsonl` 에서 로드

---

## 0. 공통 전제 (모든 Wave 공유)

### 0.1 환경 변수 / 설정 키
```bash
export HEXA=$HOME/Dev/hexa-lang/target/release/hexa
export AG=$HOME/Dev/airgenome
export CFG=$AG/nexus/shared/gate_config.jsonl
```

`gate_config.jsonl` 에 존재해야 하는 키 (Wave 0 에서 주입):
| key | default | 용도 |
|---|---|---|
| `ssh_alias` | `ubu` | SSH alias (이미 존재) |
| `remote_dir` | `/home/aiden/airgenome` | 원격 작업 디렉토리 (이미 존재) |
| `remote_host` | `192.168.50.119` | IP (이미 존재) |
| `ubu_tmpfs` | `/mnt/ramdisk/airgenome` | 신규 — tmpfs 루트 |
| `ubu_python` | `python3` | 신규 — python 심볼릭 없음 |
| `ubu_gpu_worker_dir` | `/home/aiden/airgenome` | 신규 — gpu_*.py 위치 |
| `ubu_ring_slots` | `65536` | Wave 1 — 링버퍼 슬롯 수 |
| `ubu_vram_budget_mb` | `1024` | Wave 2 — 6축 게이트 VRAM 한도 |
| `ubu_arrow_budget_mb` | `8192` | Wave 3 — Arrow 인메모리 한도 |
| `ubu_forge_idle_watts` | `20` | Wave 4 — GPU 유휴 판정 W |

### 0.2 설정 로더 불변 패턴
모든 Hexa 파일은 `load_cfg(key, fallback)` 로만 접근. 코드에 IP/경로 리터럴 금지.

### 0.3 Fallback 정책 (즉시 degrade)
```
ubu_bridge::health() == false
  ↓
  1. stdout/dispatch.log WARN
  2. growth_bus.jsonl append {"type":"fallback","phase":"ubu_offline",...}
  3. compute_target = "local"
  4. menubar 상태 → DEGRADED
  5. 60초 재시도 (큐잉 없음, 즉시 local 진행)
```

---

# Wave 0 — ubu_bridge 검증 완료 + 설정 반영

> **목표**: `ubu_bridge::health()` 가 실기계에서 `true` 를 반환. 스캐폴딩(이미 존재)을
> 실제 SSH/torch 기반으로 단단하게 만든다. L0 편집은 하지 않음.
> **소요**: 1일

## 준비물
- [ ] Mac ↔ ubu SSH 키 기반 접속 가능 (`ssh -o BatchMode=yes ubu echo ok` 성공)
- [ ] `gate_config.jsonl` 존재 (`$AG/nexus/shared/gate_config.jsonl`)
- [ ] `modules/ubu_bridge.hexa` 스캐폴딩 존재 (현재 상태)
- [ ] ubu 측 `~/airgenome/gpu_sweep.py` / `gpu_batch.py` / `gpu_cross_sweep.py` 가동 중
- [ ] ubu 측 PyTorch 2.11+cu130 import 가능

## 작업 목록 (순서)
1. **설정 키 3종 추가** — `gate_config.jsonl` 에 `ubu_tmpfs`, `ubu_python`, `ubu_gpu_worker_dir` append
2. **health() 실측 테스트** — 스캐폴딩 함수 그대로 실행, 실패 시 진단
3. **exec() / exec_python() 평탄화** — `contains("exec error")` 가정은 약함. exit code 기반 감시로 교체 (SSH 리턴 + `echo $?`)
4. **fallback_local() 본문 완성** — 현재 TODO 상태. stdout + dispatch.log + growth_bus append 3단 구현
5. **absolute_rules.json AG3 블록** — `/Users/ghost/Dev/nexus/shared/absolute_rules.json` airgenome 섹션에 AG3 추가
6. **core-lockdown.json 등록** — airgenome L0 배열에 `modules/ubu_bridge.hexa` 추가 (잠금은 아님, 참조만)
7. **health() CI 스모크** — `$HEXA -e 'import ubu_bridge; println(ubu_bridge::health())'` 한 줄 실행기 준비

## 변경 파일
| 파일 | 변경 유형 |
|---|---|
| `nexus/shared/gate_config.jsonl` | append 3줄 |
| `modules/ubu_bridge.hexa` | fallback_local 본문 + exec 에러 감시 강화 |
| `/Users/ghost/Dev/nexus/shared/absolute_rules.json` | airgenome 섹션 AG3 추가 |
| `/Users/ghost/Dev/nexus/shared/core-lockdown.json` | airgenome L0 참조 목록에 ubu_bridge 등록 |
| `dispatch.log` | (자동 생성, 커밋 제외) |

## 검증 명령
```bash
# 1) 설정 로딩 확인
grep -E 'ubu_tmpfs|ubu_python|ubu_gpu_worker_dir' $CFG

# 2) SSH / torch 기본
ssh -o ConnectTimeout=3 -o BatchMode=yes ubu echo ok
ssh ubu python3 -c 'import torch; print(torch.cuda.is_available(), torch.version.cuda)'

# 3) tmpfs 존재 확인
ssh ubu 'df -h /mnt/ramdisk && ls -la /mnt/ramdisk'

# 4) health() 스모크 (Hexa)
$HEXA -e 'import modules::ubu_bridge as u; println(u::health())'
# 기대: true

# 5) 의도적 실패 — alias 오타
$HEXA -e 'import modules::ubu_bridge as u; println(u::health())' UBU_ALIAS=ubu-nope
# 기대: false + growth_bus.jsonl 에 fallback 이벤트 1건 append

# 6) growth_bus 기록 확인
tail -1 $HOME/Dev/nexus/shared/growth_bus.jsonl | grep fallback
```

## 롤백
```bash
git -C $AG restore modules/ubu_bridge.hexa nexus/shared/gate_config.jsonl
git -C $HOME/Dev/nexus restore shared/absolute_rules.json shared/core-lockdown.json
```

## 완료 판정
- `ubu_bridge::health()` → `true`
- 의도적 실패 시 `growth_bus.jsonl` fallback 이벤트 1건 기록 확인
- 설정 파일 3종 반영 + git diff 리뷰 통과
- **블로커**: 없음

---

# Wave 1 — tmpfs 게놈 링버퍼

> **목표**: 게놈 60바이트를 `/mnt/ramdisk/airgenome/genome.ring` 에 append. Mac 로컬 디스크 쓰지 않음.
> **소요**: 2일

## 준비물
- [ ] Wave 0 완료 (health() true)
- [ ] `ubu_tmpfs` 설정 키 활성
- [ ] `src/core.hexa` L0 편집 승인 (이미 확보)
- [ ] ubu `/mnt/ramdisk` 여유 > 8 GB

## 1.1 링버퍼 포맷 정의

파일: `/mnt/ramdisk/airgenome/genome.ring`

```
[ HEADER 64B ]
  offset  size  field
  0x00    4     magic   = b"AG1R"
  0x04    2     version = 0x0001 (little-endian)
  0x06    2     slot_size = 60 (AG1 60바이트 게놈)
  0x08    4     slot_count = <ubu_ring_slots> (기본 65536)
  0x0C    4     head     (원자적 append 포인터, 0..slot_count-1)
  0x10    4     tail     (consumer reader 포인터)
  0x14    8     generation (누적 wrap 카운트)
  0x1C    4     writer_pid
  0x20    32    reserved (0x00)

[ SLOT 0 ]  60B  genome_0
[ SLOT 1 ]  60B  genome_1
...
[ SLOT N-1 ] 60B genome_{N-1}
```

**총 크기** = 64 + 60 × slot_count. 기본값(65536 슬롯) ≈ **3.84 MB** (tmpfs 16 GB 대비 0.024%).

**원자성**: `head` 는 python `fcntl.flock` + `struct.pack_into` 로만 갱신 (writer 단일 스레드 전제). Mac 은 writer 만, ubu consumer 는 tail 만 건드린다.

## 1.2 L0 편집 — `src/core.hexa::sample()`

현재 시그니처 (라인 수정 전 `grep -n "fn sample" src/core.hexa` 로 실제 라인 재확인):
```
fn sample() -> Genome
```

변경:
```
fn sample(compute_target: str) -> Genome    # 기본값은 호출부에서 "ubu"
```

**분기 삽입 위치**: 함수 첫 블록 — 로컬 `ps/vm_stat/iostat` 호출 직전에
```
if compute_target == "ubu" {
    let raw = ubu_bridge::exec_python(PS_SCRIPT)   // ps 스크립트는 기존 로직을 python 로 포팅
    return Genome::from_raw(raw)
}
// 아래부터 기존 로컬 경로 (fallback)
```

**호출부 수정**: `run.hexa` / `modules/guard.hexa` 의 `sample()` 호출부를 `sample("ubu")` 로 교체.

> 주의: `compute_target` 은 hard-default 금지. `run.hexa` 가 `load_cfg("default_compute_target", "ubu")` 결과를 넘겨야 함.

## 1.3 Mac → ubu append 경로

`modules/ubu_bridge.hexa` 에 추가:
```
fn ring_append(genome60: bytes) -> int
```
본문 요지 (python3 -c 원격 실행):
```python
import os, struct, fcntl
PATH = os.environ["UBU_TMPFS"] + "/genome.ring"
SLOT = 60
HDR  = 64
with open(PATH, "r+b") as f:
    fcntl.flock(f, fcntl.LOCK_EX)
    f.seek(0); magic, ver, sz, n, head, tail, gen, pid = struct.unpack("<4sHHIIIQI", f.read(32))
    off = HDR + head * SLOT
    f.seek(off); f.write(<stdin_bytes>)
    head = (head + 1) % n
    if head == 0: gen += 1
    f.seek(0x0C); f.write(struct.pack("<I", head))
    f.seek(0x14); f.write(struct.pack("<Q", gen))
    fcntl.flock(f, fcntl.LOCK_UN)
print(head)
```

## 1.4 ubu 측 consumer 스크립트 (신규)

파일: `~/airgenome/ring_consumer.py` (ubu)
```python
#!/usr/bin/env python3
# Wave 1 — genome.ring tail consumer
# 용도: tail→head 까지 슬롯을 읽어 stdout JSON 라인으로 뱉음 (gpu_sweep 의 입력)
import os, struct, sys, time, fcntl, json, base64
PATH = os.environ.get("RING_PATH", "/mnt/ramdisk/airgenome/genome.ring")
HDR, SLOT = 64, 60
with open(PATH, "rb") as f:
    fcntl.flock(f, fcntl.LOCK_SH)
    hdr = f.read(HDR)
    magic, ver, sz, n, head, tail, gen, pid = struct.unpack_from("<4sHHIIIQI", hdr, 0)
    assert magic == b"AG1R"
    cur = tail
    while cur != head:
        f.seek(HDR + cur * SLOT)
        blob = f.read(SLOT)
        sys.stdout.write(json.dumps({"slot": cur, "b64": base64.b64encode(blob).decode()}) + "\n")
        cur = (cur + 1) % n
    fcntl.flock(f, fcntl.LOCK_UN)
```
**초기화 스크립트** (`~/airgenome/ring_init.py`) — 존재하지 않을 때만 zero-fill 64 + 60×N 바이트.

## 준비물 (세부)
- [ ] `src/core.hexa` L0 편집 승인서 링크 (유저 확보 완료)
- [ ] `ubu_ring_slots` 기본값 65536 합의

## 변경 파일
| 파일 | 변경 |
|---|---|
| `src/core.hexa` | `fn sample()` → `fn sample(compute_target)` + 분기 (L0) |
| `modules/ubu_bridge.hexa` | `ring_append()` / `ring_init()` 추가 |
| `run.hexa` | `sample()` 호출부 → `sample(load_cfg("default_compute_target","ubu"))` |
| `nexus/shared/gate_config.jsonl` | `ubu_ring_slots` / `default_compute_target` 추가 |
| `~/airgenome/ring_consumer.py` (ubu) | 신규 |
| `~/airgenome/ring_init.py` (ubu) | 신규 |

## 검증 명령
```bash
# 1) 링버퍼 초기화
ssh ubu 'mkdir -p /mnt/ramdisk/airgenome && python3 ~/airgenome/ring_init.py'
ssh ubu 'ls -la /mnt/ramdisk/airgenome/genome.ring && stat -c %s /mnt/ramdisk/airgenome/genome.ring'
# 기대: 64 + 60*65536 = 3932224

# 2) Mac 에서 1회 sample → 링버퍼 head 증가 확인
$HEXA $AG/run.hexa sample-once
ssh ubu 'python3 -c "import struct;f=open(\"/mnt/ramdisk/airgenome/genome.ring\",\"rb\");h=f.read(32);print(struct.unpack_from(\"<4sHHIIIQI\",h,0))"'
# 기대: head=1

# 3) consumer 동작
ssh ubu 'python3 ~/airgenome/ring_consumer.py | head -5'
# 기대: {"slot":0,"b64":"..."} 라인 1개 (첫 게놈)

# 4) Mac 로컬 디스크 쓰기 0 확인
fs_usage -w -f filesystem hexa 2>&1 | grep -v ramdisk | grep -i genome  # 아무 것도 안 나와야 함

# 5) 100회 append → head wrap 테스트
for i in {1..100}; do $HEXA $AG/run.hexa sample-once; done
ssh ubu 'python3 ~/airgenome/ring_consumer.py | wc -l'
# 기대: 100
```

## 롤백
```bash
git -C $AG restore src/core.hexa run.hexa modules/ubu_bridge.hexa nexus/shared/gate_config.jsonl
ssh ubu 'rm -f /mnt/ramdisk/airgenome/genome.ring ~/airgenome/ring_consumer.py ~/airgenome/ring_init.py'
```

## 완료 판정
- 100회 sample 후 ubu head = 100, Mac 로컬 디스크 쓰기 0 바이트
- health() false 시 sample() 이 local 경로로 떨어지고 warning 기록
- **예상 블로커**: hexa 바이트 타입 미지원 → base64 str 로 우회

---

# Wave 2 — PyTorch 6축 게이트 병렬화

> **목표**: 15쌍 게이트 연산을 torch 텐서 1-pass. Mac 로컬 CPU 사용 0.
> **소요**: 2일

## 준비물
- [ ] Wave 1 완료 (링버퍼 동작)
- [ ] W0-D agent 결과 — **`gpu_sweep.py` 확장 결정** (sweep = 기존에 batch loop 가 이미 있음, 가장 확장성 높음)
- [ ] `ubu_vram_budget_mb` 설정 키 (기본 1024)

## 2.1 확장 파일 결정

**`~/airgenome/gpu_sweep.py`** 를 확장 (W0-D 결과).
- 이유: 기존 `--mode=sweep` 플래그 존재, argparse 구조, torch import 이미 완료.
- `gpu_batch.py` 는 단일 배치 고정, `gpu_cross_sweep.py` 는 교차 연산 전용 → Wave 3 용으로 남김.

## 2.2 15쌍 게이트 1-pass 구현

6축 = [cpu, ram, swap, net, disk, gpu]. 쌍 = C(6,2) = **15쌍**.
입력 텐서 shape: `(B, 6)` float32 — B=배치 크기(프로세스 수).

```python
# gpu_sweep.py 내 신규 함수
def six_axis_gate(x: torch.Tensor) -> torch.Tensor:
    # x: (B, 6)  -> out: (B, 15)  각 쌍의 게이트 값 (AG1 15쌍 정의)
    idx_a = torch.tensor([0,0,0,0,0,1,1,1,1,2,2,2,3,3,4], device=x.device)
    idx_b = torch.tensor([1,2,3,4,5,2,3,4,5,3,4,5,4,5,5], device=x.device)
    a = x.index_select(1, idx_a)  # (B,15)
    b = x.index_select(1, idx_b)
    # 게이트 함수 = min(a,b) * sqrt(a*b) (AG1 정의 — forge.hexa 의 gate_eval 과 일치)
    return torch.minimum(a, b) * torch.sqrt(a * b + 1e-9)
```

**argparse 플래그**: `--mode=six_axis_gate --in=<ring_path> --out=<result_path>`
- 입력: `ring_consumer.py` 의 stdout 을 stdin 으로 파이프 (base64 → float32 × 6)
- 출력: `/mnt/ramdisk/airgenome/gate_result.bin` (B × 15 × 4B)

**VRAM 1 GB 한도**: B × 6 × 4 + B × 15 × 4 ≈ B × 84B. B=10M 이면 840 MB. 기본 배치 5M 사용. `torch.cuda.memory_reserved()` 후 budget 초과 시 chunk 분할.

## 2.3 Mac → ubu 디스패치

`modules/forge.hexa::forge()` 에 삽입:
```
let out = ubu_bridge::gpu_submit("gpu_sweep.py", "--mode=six_axis_gate --in=/mnt/ramdisk/airgenome/genome.ring --out=/mnt/ramdisk/airgenome/gate_result.bin")
```
결과는 `ubu_bridge::tmpfs_read("gate_result.bin")` 로 회수, 로컬 파싱.

## 2.4 결과 일치 검증 스크립트 (신규)

`tests/w2_parity.hexa` — 동일 입력 10개 프로세스 → (a) Mac local `forge.hexa::gate_eval`, (b) ubu `six_axis_gate` → diff < 1e-5.

## 변경 파일
| 파일 | 변경 |
|---|---|
| `~/airgenome/gpu_sweep.py` (ubu) | `--mode=six_axis_gate` 추가 |
| `modules/forge.hexa` | `forge()` → `ubu_bridge::gpu_submit` 경유 (L0 편집) |
| `tests/w2_parity.hexa` | 신규 일치 검증 |
| `nexus/shared/gate_config.jsonl` | `ubu_vram_budget_mb` 추가 |

## 검증 명령
```bash
# 1) ubu 단독 실행
ssh ubu 'cd ~/airgenome && python3 gpu_sweep.py --mode=six_axis_gate \
  --in=/mnt/ramdisk/airgenome/genome.ring \
  --out=/mnt/ramdisk/airgenome/gate_result.bin'
ssh ubu 'ls -la /mnt/ramdisk/airgenome/gate_result.bin'

# 2) VRAM 사용 확인
ssh ubu 'nvidia-smi --query-gpu=memory.used --format=csv,noheader'
# 기대: < 1024 MiB

# 3) 일치 검증
$HEXA $AG/tests/w2_parity.hexa
# 기대: "PASS: max_diff=<1e-5"

# 4) 1000 샘플 배치 타이밍
time $HEXA $AG/run.hexa forge --batch=1000
# 기대: < 100ms wall time (SSH 왕복 포함 — 허용치 200ms)

# 5) Mac CPU 사용률
top -l 1 -pid $(pgrep hexa | head -1) | tail -3
# 기대: CPU < 5%
```

## 롤백
```bash
git -C $AG restore modules/forge.hexa nexus/shared/gate_config.jsonl
ssh ubu 'cd ~/airgenome && git checkout gpu_sweep.py'  # ubu 리포가 있으면
# 또는 gpu_sweep.py 백업본 복구
```

## 완료 판정
- Mac local vs ubu GPU 결과 max diff < 1e-5
- 1000 프로세스 배치 wall time < 200ms (SSH 포함)
- VRAM < 1 GB, Mac CPU < 5%
- **예상 블로커**: torch `index_select` 에 long tensor 요구 → `.long()` 명시. PyTorch 2.11 API 확인.

---

# Wave 3 — OLAP + 유사도 + LLM

> **목표**: 대규모 게놈 검색/분석 인프라.
> **소요**: 3일 (서브웨이브 3a/3b/3c)

## 준비물
- [ ] Wave 2 완료
- [ ] ubu 측 pyarrow 설치 가능 여부 사전 확인 (`ssh ubu python3 -c 'import pyarrow'` → 실패 시 `pip install --user pyarrow`)
- [ ] llama.cpp 사전 빌드 바이너리 배치 경로 합의 (`~/airgenome/llama/main`)
- [ ] `ubu_arrow_budget_mb` 설정 키

---

## Wave 3a — Arrow OLAP (in-memory 8 GB)

**작업**
1. `~/airgenome/arrow_store.py` 신규 — mmap 기반 Arrow 테이블 생성 (`pyarrow.Table.from_arrays`)
2. 스키마: `slot:int32, generation:int64, axis0..5:float32, gate0..14:float32, ts:int64`
3. 8 GB 상한: row = 4+8+6*4+15*4+8 = 104B → **약 80M rows** 수용. 초과 시 oldest eviction.
4. `ubu_bridge::arrow_push(slots)` / `arrow_query(sql_like)` 추가
5. DuckDB 사용 선택지 검토 — ubu 에 없으면 pyarrow.compute 만 사용

**변경 파일**
- `~/airgenome/arrow_store.py` (신규)
- `modules/ubu_bridge.hexa` — `arrow_push/arrow_query`
- `nexus/shared/gate_config.jsonl` — `ubu_arrow_budget_mb=8192`

**검증**
```bash
# 10M row 삽입
ssh ubu 'python3 -c "
import pyarrow as pa, numpy as np, os
n=10_000_000
t=pa.table({'slot':np.arange(n,dtype='i4'),'g':np.zeros(n,dtype='i8'),
            **{f'a{i}':np.random.rand(n).astype('f4') for i in range(6)},
            **{f'g{i}':np.random.rand(n).astype('f4') for i in range(15)},
            'ts':np.zeros(n,dtype='i8')})
print(t.nbytes/1e9,'GB')
"'
# 기대: ~1.04 GB (10M rows), 8GB 한도 내
```

**완료 기준**: 10M row 삽입 < 3s, 쿼리 (단일 axis 범위 필터) < 200ms, 메모리 < 8 GB

---

## Wave 3b — 유사도 cosine N×N

**작업**
1. `~/airgenome/gpu_cross_sweep.py` 확장 — `--mode=cosine_nxn --n=<N>`
2. `torch.nn.functional.cosine_similarity` — (N,D) × (N,D).T → (N,N)
3. W0-B T7 벤치 결과 참조: N=10 000 기준 VRAM 약 400 MB (10000×10000×4B = 400MB)
4. 청크 처리: N > 15000 시 row-block 분할
5. 결과 Top-K (K=32) 만 반환 (N×N 전체 반환 금지 — SSH 대역폭 절약)

**검증**
```bash
time ssh ubu 'cd ~/airgenome && python3 gpu_cross_sweep.py --mode=cosine_nxn --n=10000'
# 기대: < 2s wall, VRAM < 1 GB
```

**완료 기준**: N=10 000 cosine 완료 < 2s, Top-32 결과 크기 < 1 MB 전송

---

## Wave 3c — LLM 7B Q4

**작업**
1. ubu 에 `~/airgenome/llama/main` (사전 빌드) + `~/airgenome/llama/models/7B-Q4.gguf` 배치
   - cmake 없으므로 소스 빌드 금지 — **미리 빌드한 바이너리만** 복사
   - 다운로드 경로는 유저가 수동 배치 (PR 범위 외)
2. `~/airgenome/llm_query.py` 신규 — `subprocess.run([LLAMA_MAIN, "-m", MODEL, "-p", prompt, "-n", "256"])`
3. `ubu_bridge::gpu_submit("llm_query.py", "--prompt=<...>")`
4. VRAM ≈ 5 GB (7B Q4 + context) → six_axis_gate 1GB + llm 5GB = 6GB ≤ 12GB (여유 6GB)

**검증**
```bash
ssh ubu 'ls -la ~/airgenome/llama/main ~/airgenome/llama/models/*.gguf'
time ssh ubu 'python3 ~/airgenome/llm_query.py --prompt="Hello" --n=256'
# 기대: < 10s, VRAM < 6 GB
```

**완료 기준**: 256 토큰 응답 < 10s, VRAM < 6 GB, 동시 six_axis_gate 호출 가능 (OOM 없음)

**예상 블로커**: 사전 빌드 바이너리 미확보 시 Wave 3c 연기 (3a/3b 는 독립 진행 가능).

---

# Wave 4 — 연속 돌파 루프

> **목표**: Mac 없이도 ubu 가 자율적으로 sample → gate → 이상치 → growth_bus → blowup 순환.
> **소요**: 2일

## 준비물
- [ ] Wave 1~3 완료 (링버퍼 + 게이트 + 유사도)
- [ ] ubu systemd user 서비스 권한 확인 (`systemctl --user` 또는 root 유닛)
- [ ] `ubu_forge_idle_watts` 설정 키 (기본 20)

## 4.1 파이프라인 데몬

**신규 유닛**: `/etc/systemd/system/airgenome-ag3-loop.service` (ubu)
```ini
[Unit]
Description=airgenome AG3 continuous breakthrough loop
After=network-online.target airgenome-fill.service
Wants=airgenome-fill.service

[Service]
Type=simple
User=aiden
WorkingDirectory=/home/aiden/airgenome
Environment=RING_PATH=/mnt/ramdisk/airgenome/genome.ring
ExecStart=/usr/bin/python3 /home/aiden/airgenome/ag3_loop.py
Restart=always
RestartSec=5
Nice=10

[Install]
WantedBy=multi-user.target
```

**루프 로직** (`~/airgenome/ag3_loop.py`):
```
while True:
    1. ring_consumer → 신규 슬롯 수집 (tail→head)
    2. gpu_sweep.py six_axis_gate 로 게이트값 계산
    3. 이상치 탐지: gate > mean + 3σ
    4. 이상치 발생 → growth_bus.jsonl append {type:breakthrough,phase:ag3_loop,...}
    5. 이상치 발생 → blowup.hexa 트리거 (nexus SSH)
    6. arrow_store.arrow_push(new rows)
    7. sleep(2s)
```

**blowup 트리거** (ubu 내에서):
```bash
HEXA=$HOME/Dev/hexa-lang/target/release/hexa
$HEXA $HOME/Dev/nexus/mk2_hexa/native/blowup.hexa airgenome 3 --no-graph
```

## 4.2 유휴 GPU forge

**선택**: **idle detection** (cron 아님 — 반응성 우선)

`ag3_loop.py` 내 서브태스크:
```python
def maybe_idle_forge():
    w = float(nvidia_smi_power())
    if w < CFG.ubu_forge_idle_watts and idle_duration() > 300:
        subprocess.Popen(["python3", "gpu_sweep.py", "--mode=forge_idle"])
```
- 5분 연속 유휴(<20W) → forge 가동
- forge 작업 시작 시 flag 파일 `/mnt/ramdisk/airgenome/forge.lock` 생성
- 다른 heavy job 진입 시 lock 확인 후 forge SIGTERM (Prime Directive 위배 아님 — 자기 생성 프로세스 정리)

## 변경 파일
| 파일 | 변경 |
|---|---|
| `~/airgenome/ag3_loop.py` (ubu) | 신규 |
| `~/airgenome/systemd/airgenome-ag3-loop.service` | 신규 |
| `/etc/systemd/system/airgenome-ag3-loop.service` (ubu) | 심볼릭 |
| `modules/ubu_bridge.hexa` | `loop_status()` / `loop_tail_log()` |
| `nexus/shared/gate_config.jsonl` | `ubu_forge_idle_watts=20` |

## 검증 명령
```bash
# 1) 유닛 설치
ssh ubu 'sudo cp ~/airgenome/systemd/airgenome-ag3-loop.service /etc/systemd/system/ && \
         sudo systemctl daemon-reload && \
         sudo systemctl enable --now airgenome-ag3-loop.service'

# 2) 상태
ssh ubu 'systemctl status airgenome-ag3-loop.service --no-pager'
ssh ubu 'journalctl -u airgenome-ag3-loop.service -n 50 --no-pager'

# 3) Mac shutdown 후 루프 지속 확인
# (Mac 전원 OFF 시뮬레이션 — 5분 대기)
ssh ubu 'tail -20 ~/Dev/nexus/shared/growth_bus.jsonl | grep ag3_loop'
# 기대: phase=ag3_loop 이벤트 계속 append

# 4) 유휴 forge 트리거
ssh ubu 'nvidia-smi --query-gpu=power.draw --format=csv,noheader'
# 5분 < 20W 대기 → forge.lock 생성 확인
ssh ubu 'ls -la /mnt/ramdisk/airgenome/forge.lock'

# 5) blowup 트리거 확인
ssh ubu 'grep breakthrough ~/Dev/nexus/shared/growth_bus.jsonl | tail -5'
```

## 롤백
```bash
ssh ubu 'sudo systemctl disable --now airgenome-ag3-loop.service && \
         sudo rm /etc/systemd/system/airgenome-ag3-loop.service && \
         sudo systemctl daemon-reload'
ssh ubu 'rm -f ~/airgenome/ag3_loop.py ~/airgenome/systemd/airgenome-ag3-loop.service \
              /mnt/ramdisk/airgenome/forge.lock'
git -C $AG restore modules/ubu_bridge.hexa nexus/shared/gate_config.jsonl
```

## 완료 판정
- Mac 전원 OFF 후 10분간 `ag3_loop.service` 동작 유지, growth_bus 에 이벤트 최소 5건 append
- 5분 GPU 유휴 → forge 자동 가동, forge.lock 생성
- 이상치 1건 이상 발생 시 blowup.hexa 호출 기록
- **예상 블로커**: sudo 비밀번호 프롬프트 → sudoers NOPASSWD 엔트리 필요 (유저 확인)

---

## 부록 A — 예상 블로커 총정리

| # | 블로커 | Wave | 대응 |
|---|---|---|---|
| B1 | `nvcc` 없음 | 전체 | PyTorch 연산자만. CUDA 커널 직접 작성 금지 |
| B2 | `cmake` 없음 | 3c | llama.cpp 사전 빌드 바이너리 수동 배치 |
| B3 | `python` 심볼릭 없음 | 전체 | `ubu_python` 설정 키 → `python3` |
| B4 | hugepages 할당 0 | 1~3 | tmpfs 만 사용. W4 이후 재검토 |
| B5 | `hexa` PATH 미등록 | 전체 | `$HOME/Dev/hexa-lang/target/release/hexa` full path |
| B6 | `pyarrow` 미설치 가능성 | 3a | 사전 `pip install --user pyarrow` |
| B7 | sudo NOPASSWD 미설정 | 4 | systemd 설치 시 유저 승인 필요 |
| B8 | SSH 왕복 레이턴시 | 2 | 배치 호출로 amortize, 청크 최소 1000 |
| B9 | tmpfs 16 GB 한도 | 1, 3a | Arrow 8 GB + ring 4 MB + llama 6 GB cap < 16 GB 검증 |
| B10 | VRAM 12 GB 한도 | 2, 3b, 3c | six_axis 1 GB + cosine 1 GB + llm 5 GB = 7 GB ≤ 12 GB |

## 부록 B — 하드코딩 금지 재확인

모든 Wave 의 모든 파일에서:
1. IP / 호스트명 → `load_cfg("ssh_alias", ...)` / `load_cfg("remote_host", ...)`
2. 경로 → `load_cfg("remote_dir", ...)` / `load_cfg("ubu_tmpfs", ...)` / `load_cfg("ubu_gpu_worker_dir", ...)`
3. Python 바이너리 → `load_cfg("ubu_python", "python3")`
4. 수치 상한 → `load_cfg("ubu_vram_budget_mb", ...)` / `load_cfg("ubu_arrow_budget_mb", ...)` / `load_cfg("ubu_forge_idle_watts", ...)`

코드 리뷰 체크리스트: `grep -nE '192\.168|/mnt/ramdisk|/home/aiden|ubu[^_]' modules/ src/ ~/airgenome/*.py` 결과가 **0 라인** 이어야 함 (주석 제외).

## 부록 C — 진행 승인 흐름

```
Wave 0 완료 → 유저 리뷰 → Wave 1 L0 편집 승인 (core.hexa sample 시그니처)
Wave 1 완료 → 유저 리뷰 → Wave 2 L0 편집 승인 (forge.hexa forge() 경유)
Wave 2 완료 → 유저 리뷰 → Wave 3 승인 (3a/3b/3c 순차)
Wave 3 완료 → 유저 리뷰 → Wave 4 승인 (systemd sudo 요구)
Wave 4 완료 → blowup.hexa airgenome 3 → ossified 승격 심사
```
