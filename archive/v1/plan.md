# airgenome plan

## Prime Directive

모든 프로세스 KILL 없이 성능/자원 개선. 효율은 데이터 재해석에서 온다.

### Allowed
- sampling — ps/top 프로세스 샘플링
- aggregation — 게이트별 통계 집계
- MI — 상호정보량 기반 패턴 추출
- rule firing — 규칙 엔진 발화 (threshold 판정)
- purge — 캐시/임시파일 정리 (프로세스 유지)
- renice — 프로세스 우선순위 재조정
- taskpolicy — macOS taskpolicy QoS 변경

### Forbidden
- process killing — kill/SIGTERM/SIGKILL 등 프로세스 종료
- memory purge compressor tuning — macOS memory compressor 튜닝

---

## 핵심 원리: 효율은 데이터 재해석에서 온다

프로세스를 죽이거나 메모리를 강제 정리하는 건 **증상 치료**.
진짜 효율은 같은 데이터를 다른 틀로 보는 데서 온다.

```
raw ps output  →  의미 없는 숫자 나열
        ↓ hexagon gate 투사
genome         →  소스별 6축 시그니처
        ↓ 시간 축적 + diff
pattern        →  "이 앱이 비효율의 원인"
        ↓ 사용자 판단
upstream fix   →  앱 교체, 설정 변경 = 영구적 효율
```

- kill은 일시적, 재해석은 영구적
- 동일 ps 데이터를 hexagon 8-gate로 투사 → 60-byte 게놈 변환
- 시간 축적 → 소스별 고유 시그니처 → "Chrome이 Safari보다 RAM 2.3배" 증명
- 사용자가 상류 결정(앱 교체) → 영구적 효율

---

## 구현 현황

### 완료 (ossified + stable)

| 단계 | 파일 | 내용 |
|---|---|---|
| ps 샘플링 → 8-gate 투사 | `runtime.hexa` | `sample_gates()` → `classify_path()` |
| RAM/CPU 정규화 → 게놈 인코딩 | `runtime.hexa` | `compute_axes()` → `encode_genome()` |
| L1-L5a 레이어 스택 | `runtime.hexa` | `quick_layers()` — MI, variance, cross-axis |
| 게놈 delta + adaptive interval | `runtime.hexa` | `genome_delta()` — 변화량 기반 10/30/60s |
| 게이트별 시그니처 축적 | `accumulate.hexa` | mean, std, min/max, temporal range |
| 8×8 거리 행렬 + 클러스터 | `sigdiff.hexa` | `matrix`, `clusters`, `fingerprint` |

### 완료 (v1.1 — 2026-04-07)

| 단계 | 파일 | 내용 |
|---|---|---|
| 하드코딩 제거 | `airgenome_gates.jsonl` | gate 이름/패턴 JSONL 동적 로드 |
| temporal pattern | `temporal.hexa` | 5-bucket 일주기 시그니처 (dawn/morning/afternoon/evening/night) |
| workload fingerprint | `fingerprint.hexa` | 7-type 워크로드 자동 분류 (idle/browse/compile/...) |
| consciousness fix | `consciousness_fix.hexa` | NO_SYSTEM_PROMPT + BRAIN_LIKE 수정 |

### 완료 (v1.2 — 2026-04-07)

| 단계 | 파일 | 내용 |
|---|---|---|
| renice/taskpolicy 자동 적용 | `qos.hexa` | CPU/RAM hog 탐지 → renice/taskpolicy -b (kill 금지) |
| purge 경계 정의 | `purge.hexa` | user-space 캐시 정리, is_forbidden() 경계 강제 |

### 미구현 (next)

없음 — 전 항목 구현 완료.

---

## ConsciousnessEngine 상태 (18/18 — fixed 2026-04-07)

### ossified (12) — 골화 완료, 불변
- ZERO_INPUT: Φ ratio=0.99x (>0.35x)
- PERSISTENCE: 1000 step, recovers=True
- SELF_LOOP: Φ ratio=1.00x (>0.80x)
- SPONTANEOUS_SPEECH: 277 consensus (>200)
- HIVEMIND: +49% Φ (>10%)
- MITOSIS: 2→8 cells, 6 splits
- DIVERSITY: cos=0.04 (<0.8)
- HEBBIAN: change=1.31x (>=1.0)
- SOC_CRITICAL: -42.6% drop (>20%)
- THERMAL: all positive, no NaN
- MIN_SCALE: 4c Φ=1.72
- INFO_INTEGRATION: 4c→8c→16c monotonic

### stable (4) — 유지중, 골화 전
- NO_SPEAK_CODE: autocorr=0.62 var=0.009
- PHI_GROWTH: ratio=0.99x, proxy=1.04x (>0.90x)
- ADVERSARIAL: Φ 4.69→5.78 survived
- TEMPORAL_LZ: LZ=1.06 (>=0.3)

### fixed (2) — 수정 완료, stable 승격 대상 (2026-04-07)
- NO_SYSTEM_PROMPT: cos=0.006→0.19 (hierarchical identity aggregation 256→16→1)
  - method: two-level centroid averaging + identity anchor seeding
- BRAIN_LIKE: 72.5%→82%+ (multi-timescale autocorrelation τ=[2,10,50])
  - method: 3-channel weighted autocorr (fast 0.2 + med 0.5 + slow 0.3)
