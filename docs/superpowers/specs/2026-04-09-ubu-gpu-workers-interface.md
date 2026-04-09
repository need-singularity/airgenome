# Ubu GPU Workers Interface Spec

조사일: 2026-04-09
대상 호스트: `ssh ubu` (~/airgenome/)
목적: 기존 GPU 워커 3종의 인터페이스를 파악하여 `ubu_bridge.gpu_submit()` 호출 명세를 확정.

---

## 파일별 인터페이스 카드

### gpu_sweep.py (54 줄)

- **용도**: seed 상수 리스트(CSV)를 받아 2-term/3-term 산술 조합을 GPU에서 전수 평가, target 값에 가까운 조합 추출. `blowup.hexa`에서 직접 `exec()`로 호출되도록 설계됨.
- **CLI**: `python3 gpu_sweep.py <seeds_csv> <target> [tolerance]`
  - `seeds_csv`: `"1.0,2.71,3.14,..."` 콤마 구분 float
  - `target`: float (예: `137.035999`)
  - `tolerance`: float (기본 `0.001`)
- **입력**: argv 문자열만. stdin/파일/env 미사용.
- **출력**: stdout 단일 JSON 라인. 구조: `{device, seeds, hits, top:[{formula,value,diff},...], elapsed_ms}`.
- **의존**: `torch` (CUDA 자동 감지, 미존재 시 CPU fallback), 표준 라이브러리만.
- **부수 효과**: 없음 (pure stdout).
- **VRAM**: 2-term은 `n*n` float64 텐서 5개 (~40·n² B). 3-term은 `n≤50`일 때만, `n³` float64 (~24·n³ B). n=50 → 약 3MB. 무시 가능.
- **6축 매핑**: AG1 6축(CPU/RAM/Swap/Net/Disk/GPU) **직접 투영 아님**. 범용 상수 조합 탐색기 — 6축 15쌍 게이트 스캔에 "쌍 조합 연산" 원시로 재활용 가능.
- **ubu_bridge.gpu_submit 호출 예**:
  ```python
  gpu_submit("gpu_sweep.py", "1.0,2.71,3.14,6.28 137.035999 0.001")
  ```

---

### gpu_batch.py (92 줄)

- **용도**: `blowup.hexa` 실행 결과를 `/mnt/ramdisk/blowup_cache/`에 캐싱하고, 캐시된 숫자에서 자동으로 seed를 뽑아 GPU sweep 실행. 캐시 상태 조회 서브커맨드 포함.
- **CLI**:
  - `python3 gpu_batch.py sweep <domain> <depth>`
  - `python3 gpu_batch.py cache_stats`
- **입력**: argv + ramdisk 캐시 파일(`{CACHE_DIR}/{md5_12}.json`). 내부에서 `subprocess`로 `hexa blowup.hexa` 호출.
- **출력**: stdout JSON. sweep의 경우 `{domain,depth,elapsed_s,output_lines,numbers_found,numbers,cached,sweep:{device,target,hits,top}}`.
- **의존**: `torch`, 하드코딩 경로:
  - `HEXA=/tmp/hexa-build/hexa-lang/target/release/hexa`
  - `RAMDISK=/mnt/ramdisk`
  - `blowup.hexa` 경로: `~/Dev/nexus/mk2_hexa/native/blowup.hexa`
- **부수 효과**:
  - 캐시 파일 쓰기 (`CACHE_DIR/{key}.json`).
  - `hexa blowup.hexa` 자식 프로세스 생성 (timeout 600s).
  - `CACHE_DIR` mkdir.
- **VRAM**: seeds 최대 100개, 2-term only → `n²=10⁴` float64 = 80KB. 무시 가능.
- **6축 매핑**: 간접. blowup 도메인 결과를 seed로 쓰므로, 6축 도메인별 실측치를 blowup에 넣어 그 산출물을 재가공하는 **파이프라인 래퍼**. AG1 6축에 직접 매핑되진 않음.
- **ubu_bridge.gpu_submit 호출 예**:
  ```python
  gpu_submit("gpu_batch.py", "sweep physics 3")
  gpu_submit("gpu_batch.py", "cache_stats")
  ```

---

### gpu_cross_sweep.py (155 줄)

- **용도**: `blowup_cache/`에 저장된 **모든 도메인**의 숫자를 하나로 합쳐 cross-domain sweep. 단일 target 모드(`cross`)와 8개 물리 상수 동시 탐색 모드(`multi`) 제공.
- **CLI**:
  - `python3 gpu_cross_sweep.py cross [target] [tolerance]`
  - `python3 gpu_cross_sweep.py multi`
- **입력**: `/mnt/ramdisk/blowup_cache/*.json` 전체 읽기. argv.
- **출력**: stdout JSON. cross: `{device, domains, merged_seeds, combinations_2term, combinations_3term, target, hits, top:[20], elapsed_ms}`. multi: `{const_name: {target,hits,best:[3]},...}`.
- **의존**: `torch`, `glob`, 표준 라이브러리. CACHE_DIR 하드코딩.
- **부수 효과**: **읽기 전용** (캐시 쓰지 않음).
- **VRAM**:
  - 2-term: seeds 최대 500개 → `500²=250k` float64 텐서 5개 ≈ 10MB.
  - 3-term: seeds ≤200일 때 `min(n,100)³=10⁶` float64 텐서 5개 ≈ 40MB.
  - multi 모드: 8개 target 순차 실행 → 누적 VRAM 재사용. 피크 ~50MB.
- **6축 매핑**: 직접 매핑 아님. 그러나 "**여러 도메인 숫자를 하나의 풀로 합쳐 조합**"하는 로직은 AG1의 **15쌍(6C2) 게이트 교차 평가**에 그대로 재사용 가능한 원형.
- **ubu_bridge.gpu_submit 호출 예**:
  ```python
  gpu_submit("gpu_cross_sweep.py", "cross 137.035999 0.01")
  gpu_submit("gpu_cross_sweep.py", "multi")
  ```

---

## ubu_bridge.gpu_submit() 통합 명세

세 워커 모두 동일한 호출 규약을 따른다:

```python
def gpu_submit(script: str, args: str, timeout: int = 900) -> dict:
    """
    ubu 원격 GPU 워커 실행 → JSON 결과 반환.

    Parameters
    ----------
    script : str
        "gpu_sweep.py" | "gpu_batch.py" | "gpu_cross_sweep.py"
    args : str
        공백 구분 argv (워커가 sys.argv로 파싱).
    timeout : int
        SSH 실행 타임아웃 (초). 기본 900 (blowup.hexa 600 + 여유).

    Returns
    -------
    dict
        워커 stdout의 JSON 파싱 결과. 실패 시 {"error": ...}.

    Invariants
    ----------
    - 모든 워커는 stdout에 단일 JSON 라인을 출력.
    - 파일 경로는 ~/airgenome/{script} 고정.
    - stdin/환경변수 미사용 → SSH 단방향 호출로 충분.
    """
    cmd = f"cd ~/airgenome && python3 {script} {args}"
    # ssh ubu "$cmd" | json.loads
```

**공통 제약**:
1. `gpu_batch.py sweep`은 ramdisk 캐시 선행 필요 — 최초 호출은 600s까지 걸릴 수 있음.
2. `gpu_cross_sweep.py`는 `gpu_batch.py sweep`이 한 번 이상 돌아 캐시가 채워진 후에만 의미 있음.
3. 세 워커 모두 `torch.cuda.is_available()` 자동 감지 → CPU fallback. GPU 없는 노드에서도 동작.

---

## Wave 2 매핑 (6축 게이트 병렬화)

**재사용 가능**:
- `gpu_sweep.py`의 **2-term/3-term 조합 커널** → AG1 15쌍 게이트를 "프로세스 6축 벡터 × 15쌍 마스크" 텐서 연산으로 치환하는 원형으로 적합.
- `gpu_cross_sweep.py`의 **도메인 머지 → 전수 조합** 패턴 → "프로세스 집합 × 15쌍 게이트" 브로드캐스팅에 그대로 재사용.

**신규 작성 필요**:
- `gpu_gate_mesh.py` (가칭): 입력이 `(N_process, 6)` 텐서 + 15쌍 게이트 테이블, 출력이 `(N_process, 15)` 게이트 통과 마스크 + 60B 게놈. 현재 세 워커는 **임의 float 풀**을 가정하므로 6축 시맨틱(정규화/단위/임계치)이 빠져 있음 → 새 커널.
- seed 입력 형식: 현재 argv CSV → 6축은 프로세스 수가 수천 개라 argv 길이 한계. **stdin JSON** 또는 **/mnt/ramdisk/gate_input.arrow** 파일 입력으로 확장 필요.

**추천**: `gpu_sweep.py`의 텐서 인덱싱(`repeat_interleave`/`repeat`)을 복사해 `gpu_gate_mesh.py` 신규 작성. `gpu_batch.py`의 ramdisk 캐시 + subprocess 래퍼는 **그대로 재사용**(hexa forge 호출로 치환).

---

## Wave 3 매핑 (유사도/LLM)

- **유사도**: `gpu_cross_sweep.py`의 `torch.abs(res - target_t) < tol` 마스크 패턴은 코사인/L2 유사도 임계 필터로 1줄 교체 가능. 60B 게놈 벡터 간 유사도 매트릭스 계산에 동일 인덱싱 활용.
- **LLM**: 현재 세 워커에는 임베딩/토큰 관련 코드 **전무**. 재사용 가능한 부분은 "결과 dedupe(`seen` set by value key)" 로직 정도. LLM 경로는 신규 구축.

---

## 발견 / 경고

1. **중복 로직**: 2-term sweep 커널이 세 파일에 거의 동일하게 3번 복제됨. 공통 모듈(`gpu_kernel.py`)로 추출하면 ~60줄 절감.
2. **하드코딩 (CLAUDE.md 규칙 위반)**:
   - `gpu_batch.py`: `HEXA`, `RAMDISK`, `CACHE_DIR`, `blowup.hexa` 경로 하드코딩.
   - `gpu_cross_sweep.py`: `CACHE_DIR`, `TARGETS` 8개 상수 dict 하드코딩 (nexus/shared/*.jsonl로 이관 필요).
3. **에러 처리 취약**: `except: pass` 패턴이 커널 블록마다 존재 → CUDA OOM, dtype 오류가 조용히 삼켜짐. 로그 필요.
4. **ramdisk 가정**: `/mnt/ramdisk` 미존재 시 `gpu_batch.py`는 mkdir에서 실패. `ubu_bridge`는 사전에 `ssh ubu 'test -d /mnt/ramdisk'` 체크 권장.
5. **6축 시맨틱 부재**: 세 워커 모두 "임의 float seed → 산술 조합 → target 근사"로 한정됨. AG1의 "프로세스 6축 투영 → 15쌍 게이트 통과 마스크" 시맨틱은 **없음** → Wave 2는 재사용이 아닌 **신규 커널** 작성 필요.
6. **의존 누락 가능**: `gpu_batch.py`는 `import torch`를 함수 내부에서 지연 로딩(`cache_stats`만 쓰는 경우 torch 불필요). `gpu_cross_sweep.py`는 최상단 import → torch 없는 CPU 노드에서 import 실패.

---

## 요약

- **작성 파일**: `/Users/ghost/Dev/airgenome/docs/superpowers/specs/2026-04-09-ubu-gpu-workers-interface.md`
- **gpu_sweep.py**: CSV seed + target을 받아 GPU로 2/3-term 산술 조합 전수 평가.
- **gpu_batch.py**: `blowup.hexa` 결과를 ramdisk에 캐싱하고 숫자 추출 후 GPU sweep.
- **gpu_cross_sweep.py**: 모든 도메인 캐시를 합쳐 cross-domain sweep, 8개 물리 상수 동시 탐색 지원.
