# AG3 Policy Enforcement 설계 — "문서 규칙" → "코드 불변식"

> **작성일**: 2026-04-09
> **상위 스펙**: `2026-04-09-ubuntu-first-breakthrough-design.md` (AG3 선언)
> **목적**: AG3(Ubuntu-First)를 선언에서 **집행 가능한 런타임 불변식**으로 승격
> **상태**: SPEC — 구현 전 유저 리뷰 대기

---

## 0. 문제 정의

현재 AG3는 `absolute_rules.json` 에 선언되어 있고 `ubu_bridge.hexa` 가 존재하지만,
heavy compute 가 **우회되어도 아무도 막지 않는다**. 규칙이 "권고" 수준이다.

필요: **코드가 거부하는 불변식** — 위반 시도 시 런타임/커밋 단계에서 차단.

---

## 1. 설계 요약

### 1.1 집행 3지점 (①+③+④ 조합)

```
    ┌─────────────────────────────────────────────────┐
    │ ④ 정적 린터 (scripts/ag3_lint.hexa)              │
    │   pre-commit: 금지 패턴 (로컬 torch/cupy 등) 스캔 │
    └─────────────────┬───────────────────────────────┘
                      │ 커밋 차단
                      ▼
    ┌─────────────────────────────────────────────────┐
    │ ③ 정책 진실 소스 (nexus/shared/ag3_policy.jsonl) │
    │   heavy compute 화이트리스트 (op 단위)            │
    └─────────────────┬───────────────────────────────┘
                      │ 런타임 룩업
                      ▼
    ┌─────────────────────────────────────────────────┐
    │ ① Dispatch 가드 (run.hexa / dispatch.hexa 진입)  │
    │   ag3_guard::enforce(op) 1줄 삽입                │
    │   위반 시 mode 에 따라 strict/degrade/off        │
    └─────────────────────────────────────────────────┘
```

### 1.2 집행 모드 (Hybrid)

`gate_config.jsonl::ag3_mode` 키로 제어:

| mode | 동작 | 기본 적용 |
|---|---|---|
| `strict` | 위반 즉시 에러. fallback 금지. `--allow-local` 플래그 필수 | AG2 트리거 (Mac CPU 30%+) 자동 승격 |
| `degrade` | 위반 시 warn + growth_bus + local fallback (현행 동작) | 평상시 기본값 |
| `off` | 가드 비활성 (디버깅 전용) | 수동 전환만 |

**자동 승격 규칙**: `resource_guard` 가 Mac CPU ≥ 30% 보고 시 `ag3_guard` 가 in-memory 로
`strict` 로 부스트 (설정 파일은 건드리지 않음). 평상 복귀 시 자동 하강.

### 1.3 L0 무수정 원칙
- `src/core.hexa`, `modules/forge.hexa`, `modules/guard.hexa`, `modules/implant.hexa`,
  `modules/resource_guard.hexa` **수정 없음**.
- 가드는 **dispatch 진입점 1 곳**에서만 잡는다. heavy 함수 내부 편집 불필요.
- 상위 AG3 스펙 § 5 "L0 수정 항목" 은 **무기한 연기** — 이 스펙이 이를 대체한다.

---

## 2. 파일 변경 목록

### 2.1 신규

| 경로 | 용도 | 라인 |
|---|---|---|
| `nexus/shared/ag3_policy.jsonl` | heavy compute 화이트리스트 (진실 소스) | ~10 |
| `modules/ag3_guard.hexa` | 런타임 enforcer (얇은 레이어, L0 아님) | ~150 |
| `scripts/ag3_lint.hexa` | 정적 스캐너 (pre-commit 훅) | ~120 |

### 2.2 수정 (최소 침습)

| 경로 | 변경 | 줄 수 |
|---|---|---|
| `run.hexa` | dispatch 분기 상단에 `ag3_guard::enforce(op)` 1줄 | +1 |
| `nexus/shared/gate_config.jsonl` | `ag3_mode=degrade` 1줄 | +1 |
| `modules/ubu_bridge.hexa` | `ag3_mode` 로드 + `health()` 결과 캐시 (5초) | +15 |
| `menubar_tpl.js` | AG3 위반 카운터 (🔴 AG3:N) 표시 | +5 |
| `nexus/shared/absolute_rules.json` | AG3 enforcement 필드에 "ag3_guard/ag3_lint" 명시 | +2 |

### 2.3 무수정 (L0 보호)
- `src/core.hexa`, `modules/forge.hexa`, `modules/resource_guard.hexa`,
  `modules/guard.hexa`, `modules/implant.hexa`

---

## 3. 정책 파일 포맷 (ag3_policy.jsonl)

A+C 하이브리드: 명시 화이트리스트 + 리소스 예측 메타.

```jsonl
{"op":"six_axis_sample","module":"src/core.hexa","fn":"sample","enforce":"strict","est_vram_mb":0,"est_ram_mb":16,"notes":"6축 투영 — Ubuntu ps 기반"}
{"op":"genome_similarity","module":"modules/forge.hexa","fn":"cosine_nxn","enforce":"strict","est_vram_mb":512,"est_ram_mb":8192,"notes":"N×N Arrow OLAP"}
{"op":"forge_cycle","module":"modules/forge.hexa","fn":"forge","enforce":"strict","est_vram_mb":1024,"est_ram_mb":4096,"notes":"교차수분/돌연변이 — GPU"}
{"op":"llm_query","module":"ubu_workers/py/llama","fn":"main","enforce":"strict","est_vram_mb":5120,"est_ram_mb":2048,"notes":"llama.cpp 7B Q4"}
{"op":"implant_extract","module":"modules/implant.hexa","fn":"extract","enforce":"degrade","est_vram_mb":0,"est_ram_mb":512,"notes":"패턴 추출 — 가벼움, degrade 허용"}
```

**필드**
- `op`: 논리 연산명 (가드가 룩업하는 키)
- `module`/`fn`: 선언 위치 (린터가 검증)
- `enforce`: `strict` | `degrade` | `off` — 개별 op 레벨 오버라이드 가능
- `est_*`: 예측 리소스 (미래 동적 판정용, W1 에는 메타 전용)

**증분 규칙**: 신규 heavy op 추가 = 이 파일 한 줄. 코드 수정 0.

---

## 4. ag3_guard.hexa 인터페이스

```hexa
fn enforce(op: str) -> bool         // dispatch 진입점이 호출. 위반 시 strict=abort, degrade=warn
fn lookup(op: str) -> PolicyEntry   // ag3_policy.jsonl 룩업
fn current_mode() -> str            // ag3_mode 값 + AG2 부스트 반영
fn record_violation(op, reason)     // growth_bus.jsonl + dispatch.log
fn boost_strict_if_pressure()       // Mac CPU≥30% 감지 시 strict 승격
fn stats() -> (violations: int, last: str)  // menubar 피드용
```

로직:
```
enforce(op):
    entry = lookup(op)
    if entry == null: return true           // 화이트리스트 밖 → 검사 안 함
    mode = current_mode()                    // off | degrade | strict
    if mode == "off": return true
    ok = ubu_bridge::health_cached()
    if ok: return true                       // ubu 정상 → 통과
    record_violation(op, "ubu_offline")
    if mode == "strict":
        abort("AG3 strict: op '"+op+"' requires ubu, but ubu offline")
    return true                              // degrade: 통과하되 경고
```

---

## 5. ag3_lint.hexa (정적 린터)

**스캔 대상**: `.hexa` 소스 전체 + `ubu_workers/py/` 제외한 모든 `.py`

**금지 패턴**
| 패턴 | 이유 |
|---|---|
| Mac 쪽 `.hexa` 에서 `import torch` / `cupy` / `cudnn` | heavy compute 로컬 직접 호출 |
| Mac 쪽에서 `popen("python3 ... torch...")` | 우회 실행 |
| `forge.hexa` 에서 `ubu_bridge` 미경유 직접 루프 | 가드 우회 |
| 하드코딩된 `192.168.*` / `/home/aiden/*` / `/mnt/ramdisk/*` (ubu_bridge.hexa 제외) | R2 하드코딩 금지 |
| `ubu_workers/` 밖의 `.py` 신규 생성 | AG3 확장 조항 위반 |

**출력**: 위반 목록 → exit 1 (pre-commit 차단)
**정상**: exit 0

---

## 6. 동작 시나리오

### V1 — 평상시 (degrade)
- `run.hexa forge` → `ag3_guard::enforce("forge_cycle")` → ubu 정상 → 통과 → ubu_bridge 경유
- Mac CPU 15% → mode 유지 `degrade`

### V2 — Mac 과부하 (자동 strict 승격)
- Mac CPU 35% → `boost_strict_if_pressure()` → in-memory strict
- 이때 ubu 다운 → `enforce("forge_cycle")` → **abort**. local fallback 금지
- 사용자가 의도적으로 필요하면 `--allow-local` 플래그 명시

### V3 — ubu 다운 + 평상 (degrade)
- ubu health=false → warn + growth_bus append + 로컬 실행 (기존 동작)
- menubar 🔴 AG3:1 표시

### V4 — 금지 import 커밋 시도
- `.hexa` 에 `import torch` 추가 → `git commit` → pre-commit 훅 → ag3_lint.hexa 차단

### V5 — 신규 heavy op 추가
- `ag3_policy.jsonl` 한 줄 append → 즉시 enforce 대상. 코드 재컴파일 불필요.

---

## 7. 연쇄 돌파 훅 (향후 확장 지점)

이 스펙은 "집행" 자체에 집중하지만, 집행 통계(`ag3_guard::stats`)가 쌓이면
다음 돌파 후보가 자동으로 드러난다:

- **동적 리소스 판정**: `est_*` 실측 vs 예측 오차 → 자동 재분류
- **예측 오프로드**: 최근 violation 추세로 AG2 트리거 전에 선제 strict 부스트
- **VRAM 샤딩 스케줄러**: `ag3_policy.jsonl::est_vram_mb` 합계로 동시 실행 허용 개수 계산
- **상시 forge 데몬**: GPU 유휴 20W 미만 5분 → `forge_cycle` 자동 예약 (상위 스펙 Wave 4 와 연결)

→ 별도 세션에서 "B. 연속돌파 아이디어" 로 다룸.

---

## 8. 검증 체크리스트

| # | 항목 | 기대 |
|---|---|---|
| C1 | `ag3_guard::enforce("six_axis_sample")` 정상 호출 | true, 통과 |
| C2 | ubu 정지 + strict | abort + growth_bus 기록 |
| C3 | ubu 정지 + degrade | warn + 통과 + menubar 카운터 +1 |
| C4 | `ag3_policy.jsonl` 신규 op 한 줄 추가 | 재시작 없이 enforce 대상 |
| C5 | `.hexa` 에 금지 import 추가 후 commit | pre-commit 차단 |
| C6 | Mac CPU 35% 시뮬레이션 | in-memory strict 승격 |
| C7 | 하드코딩 금지 — ag3_guard.hexa 내 IP/경로 리터럴 0개 | 린터 통과 |

---

## 9. 본 PR 범위

1. 이 스펙 문서
2. `nexus/shared/ag3_policy.jsonl` (5 엔트리 시드)
3. `modules/ag3_guard.hexa` 스캐폴딩 (인터페이스 + 기본 구현)
4. `scripts/ag3_lint.hexa` 스캐폴딩 (금지 패턴 4 종 시작)
5. `gate_config.jsonl::ag3_mode=degrade` 추가
6. `run.hexa` 에 `ag3_guard::enforce` 1줄 삽입 — **유저 승인 후**

**제외**
- pre-commit 훅 실제 등록 (사용자 환경 결정)
- menubar 카운터 UI (후속)
- AG2 부스트 연동 (resource_guard 이벤트 구독 — 후속)

---

## 10. 리뷰 포인트

1. **자동 strict 승격 기준**: Mac CPU 30% 가 올바른 임계값인가? (AG2 와 정렬)
2. **린터 금지 패턴**: ubu_workers/py/ 외 예외 필요한가?
3. **run.hexa 1줄 삽입**: 정말 충분한가? dispatch.hexa 에도 삽입 필요?
