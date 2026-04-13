# L5c 시간지연 MI + L6e 가속도 도함수 — 돌파 설계

- **Date**: 2026-04-09
- **Owner**: airgenome
- **Status**: design + prototype
- **Related todo**: airgenome #1 (CRITICAL), #300
- **Convergence**: `shared/convergence/airgenome.json` → `failed.L5C_L6E`
- **Prime Directive safe**: read-only, 프로세스 kill 없음

## 1. 배경

`consciousness_engine_status.json` 기준 총 18 테스트 중 2개 병목:

1. `BRAIN_LIKE` — autocorr decay 65%가 천장 → 단일 lag-1 자기상관으로는 탈출 불가.
2. `NO_SYSTEM_PROMPT` (256c) — factions 다양성 과도.

`mk2_hexa/native/consciousness_fix.hexa` L187-L341 가 처방을 명시:

```
brain_like_v2 = 0.30 * L1_autocorr(lag=1)
              + 0.40 * L5c_NMI(τ=10)     — nonlinear mid-range dependence
              + 0.10 * L5c_NMI(τ=50)     — long-range memory
              + 0.20 * L6e_smoothness    — bounded-acceleration regularity
```

즉 L5c/L6e는 **육각 투영 6축 위에 얹는 시계열 차원의 2개 레이어**로, 게놈 링의 시간축을 활용해 brain-like 점수의 autocorr 천장을 돌파하는 것이 목적.

순수 6축 투영 자체는 L0(core.hexa) 불변식이며 수정 불가. L5c/L6e는 별도 모듈 (mk2_hexa + ag3 python) 에 추가한다.

## 2. 현재 자산 인벤토리

| 파일 | 기능 | 상태 |
|---|---|---|
| `mk2_hexa/native/time_delay_mi.hexa` | L5c/L6e 순수 hexa 구현 (합성 시리즈 verify) | PASS 2/2 (재실행 완료) |
| `mk2_hexa/native/real_vitals_score.hexa` | `forge/genomes.index.jsonl` 의 6축 실데이터에 L5c/L6e 적용 | 작동 (n=100 에서 GPU 축 L5c τ=1 = 0.41) |
| `mk2_hexa/native/consciousness_fix.hexa` | brain_like_v2 공식 + 처방 | 문서화 완료, 통합 대기 |
| `ag3/ring_io.py` (ubu) | 60B 게놈 링버퍼 reader | 동작 중, `iter_recent(n)` API |
| `docs/superpowers/specs/2026-04-09-genome-ring-format.md` | 링버퍼 포맷 스펙 (게놈 = 15 × f32 BE) | 확정 |

→ **재발견**: 돌파는 "구현 없음"이 아니라 "통합/검증 없음" 이었음. 프리미티브는 존재.

## 3. L5c 정의

### 3.1 수식

시간 시리즈 `X = {x_t}`, lag `τ`, bin 개수 `B`:

```
I(X_t ; X_{t-τ}) = Σ_{i,j} p_ij · log2( p_ij / (p_i · p_j) )
```

여기서 `p_ij` 는 `(x_t, x_{t-τ})` 쌍의 `B×B` 히스토그램을 정규화한 결합분포, `p_i`/`p_j` 는 주변분포.

정규화:

```
NMI(τ) = I / log2(B)        ∈ [0, 1]
```

### 3.2 알고리즘 (Python/GPU 권장)

```
def l5c_nmi(x: Tensor[N], tau: int, bins: int = 8) -> float:
    lo, hi = x.min(), x.max()
    if hi <= lo: return 0.0
    idx = ((x - lo) / (hi - lo + 1e-12) * bins).clamp(0, bins-1).long()
    xt, yt = idx[tau:], idx[:-tau]
    joint = torch.zeros(bins, bins, device=x.device)
    joint.index_put_((xt, yt), torch.ones_like(xt, dtype=joint.dtype), accumulate=True)
    p  = joint / joint.sum()
    pr = p.sum(dim=1, keepdim=True)
    pc = p.sum(dim=0, keepdim=True)
    denom = pr * pc
    mask  = (p > 1e-12) & (denom > 1e-12)
    mi    = (p[mask] * torch.log2(p[mask] / denom[mask])).sum().item()
    return max(mi / math.log2(bins), 0.0)
```

데이터 규모: 링버퍼 slot_count=65536, 현재 프로토타입은 n≤2000 권장 (CPU 만으로 <50 ms). GPU 는 선택사항, 우선 NumPy 만으로 충분.

### 3.3 입력 시그너처

`ring_io.iter_recent(n)` → 각 슬롯의 `genome` 60B → `np.frombuffer(g, dtype=">f4")` → 15차원 벡터.

축 투영은 두 가지 옵션:

- (A) **per-rail series**: 15개 레일 각각을 독립 시계열로 처리 → 15 × L5c(τ).
- (B) **per-axis aggregate**: 15 레일을 AG1의 6축 쌍 그룹(15=C(6,2))으로 aggregate → 6 × L5c.

프로토타입은 (A)로 시작 — 가장 일반적이고 `real_vitals_score.hexa` 와 직교.

## 4. L6e 정의

### 4.1 수식

```
v(t) = x(t) - x(t-1)
a(t) = x(t) - 2·x(t-1) + x(t-2)
score = clip( 1 - Var(a) / (Var(v) + ε), 0, 1 )
```

- `score ≈ 1` → 매우 매끄러운 흐름 (acceleration ≪ velocity).
- `score ≈ 0` → 지터 노이즈 (acceleration ≈ velocity).
- `Var(v) < 1e-9` → 0 반환 (flat 신호는 non-trivial 하지 않음).

(선택) jerk `j(t) = a(t) - a(t-1)` 도 계산 가능. 현재는 acceleration 레벨에서 종료.

### 4.2 구현

```
def l6e_smooth(x: Tensor[N]) -> float:
    if x.numel() < 4: return 0.0
    v = x[1:] - x[:-1]
    a = v[1:] - v[:-1]
    vv, av = v.var(unbiased=False).item(), a.var(unbiased=False).item()
    if vv < 1e-9: return 0.0
    return max(0.0, min(1.0, 1.0 - (av / (vv + 1e-9)) / 4.0))
```

(hexa 구현 `layer_l6e_acceleration_deriv` 와 1:1 대응 — 0..4 비율을 0..1 로 매핑.)

## 5. 검증 시나리오

### 5.1 합성

- `structured`: `x[i] = 0.7*(i%50)/50 + 0.3*(i%10)/10`, n=300
- `random`: xorshift/np.random, n=300

기대:
- `L5c_NMI(τ=10)` : structured ≥ 0.60, random ≤ 0.05
- `L6e`          : structured > random

현재 hexa 버전: struct=0.664 / rand=0.040 (PASS), L6e struct=0.461 / rand=0.260 (PASS).

### 5.2 실데이터

- 링에서 n=100 최근 게놈 → 레일별 τ ∈ {1,2,5,10,25,50} NMI 프로파일.
- 기대: 최소 한 개 이상의 레일에서 `τ=10` NMI > 0.15 (대부분 OS 메트릭이 수 초 단위 자기상관 보유).
- 반증: 모든 NMI < 0.05 → 링 시간 간격이 너무 뜸하거나, 샘플링이 decorrelated 해서 L5c가 신호 못 잡음 → 샘플링 주기 조정 필요.

### 5.3 brain_like_v2 통합 (다음 단계)

`consciousness_fix.hexa` 공식으로 256c 테스트 재실행 → BRAIN_LIKE 72.5% → 80% 이상 목표.
본 패치에서는 프리미티브 노출만, 통합은 후속 작업.

## 6. 기존 6축 투영과 결합

- L0 `src/core.hexa` 는 **per-slot 순간 투영** 만 담당 — 수정 없음.
- L5c/L6e 는 **링의 시간축 위 집계** → 별도 모듈 `mk2_hexa/native/real_vitals_score.hexa` (hexa 측) 와 `ag3/L5c_tdmi.py`, `ag3/L6e_jerk.py` (python/ubu 측).
- `brain_like_v2` 는 L1 autocorr + L5c + L6e 의 가중합 → 향후 `consciousness_fix.hexa` 가 집계.

## 7. 프로토타입 산출물

- `ubu:~/airgenome/ag3/L5c_tdmi.py` — 링에서 n=100 읽고 15 레일 × τ∈{1,2,5,10,25,50} NMI 테이블 출력.
- `ubu:~/airgenome/ag3/L6e_jerk.py` — 15 레일 × L6e score 출력 + 요약 (평균/최대).
- 의존: numpy 만 (PyTorch 불필요, 데이터 < 2 KB).

## 8. 블로커/추정

- **추정 1**: todo #1 의 "핵심 기능" = `BRAIN_LIKE` 천장 탈출 (= FAIL 2개 해결). `consciousness_fix.hexa` 주석과 일치.
- **추정 2**: L5c/L6e 프리미티브는 이미 PASS → 돌파 실체는 **링 → 프리미티브 → brain_like_v2 → 256c 재실행 체인**의 마지막 두 단계. 본 설계는 첫 두 단계까지만 커버.
- **유저 확인 필요**:
  1. 레일별(15) 축별(6) 중 어느 집계 선호? 기본 (A) 레일별.
  2. brain_like_v2 통합(256c 재실행) 은 별도 작업으로 분리해도 되는가?
  3. PyTorch GPU 경로 필요 여부 (현재 필요 없음 판단).

## 9. 다음 단계

1. 본 설계 승인.
2. ubu 프로토타입 실행 결과 확인 (샘플 포함 아래 커밋 메시지).
3. `brain_like_v2` hexa 통합 (별도 plan 작성).
4. 256c 의식엔진 재실행 → `BRAIN_LIKE` 갱신.
5. 통과 시 `shared/convergence/airgenome.json` → `failed.L5C_L6E` 제거, `ossified` 로 이동.
