# airgenome roadmap

> 모든 프로세스 KILL 없이 성능/자원 개선. 효율은 데이터 재해석에서 온다.

---

## v1.0 — Foundation (완료, 2026-04-05)

핵심 파이프라인: 샘플링 → 게이트 투사 → 게놈 → 축적 → 비교

| 모듈 | 파일 | 내용 |
|---|---|---|
| runtime | `runtime.hexa` | ps 샘플링 → 8-gate 투사 → 60-byte 게놈 → adaptive interval |
| accumulate | `accumulate.hexa` | 게이트별 시그니처 축적 (mean, std, min/max) |
| sigdiff | `sigdiff.hexa` | 8×8 거리 행렬, 클러스터, fingerprint |
| menubar | `run.sh` + `settings.js` | CPU/RAM/Swap 바, ceiling 슬라이더, 프로파일 자동감지 |
| forge | forge/ | 10-account Claude Code 관리 (keychain, usage, JSONL) |
| guard | guard/ | CPU/RAM/swap 모니터, 4단계 throttle (ok/warn/danger/critical) |

---

## v1.1 — Pattern Layer (완료, 2026-04-07)

재해석 레이어: 시간 패턴 + 워크로드 분류 + 하드코딩 제거

| 모듈 | 파일 | 내용 |
|---|---|---|
| temporal | `temporal.hexa` | 5-bucket 일주기 (dawn/morning/afternoon/evening/night) |
| fingerprint | `fingerprint.hexa` | 7-type 워크로드 분류 (idle/browse/compile/heavy-build/...) |
| gates JSONL | `airgenome_gates.jsonl` | gate 이름/패턴 동적 로드, 하드코딩 제거 |
| consciousness fix | `consciousness_fix.hexa` | NO_SYSTEM_PROMPT + BRAIN_LIKE → 18/18 |

---

## v1.2 — Action Layer (완료, 2026-04-07)

재해석 결과를 행동으로 연결: QoS + purge 경계

| 모듈 | 파일 | 내용 |
|---|---|---|
| qos v1 | `qos.hexa` | CPU/RAM hog → renice/taskpolicy -b (kill 금지) |
| purge | `purge.hexa` | user-space 캐시 정리, is_forbidden() 경계 강제 |
| menubar 절감률 | `run.sh` | CPU/RAM 다음 통합절감률 ↓N% 표시 |

---

## v2.0 — Smart QoS (완료, 2026-04-07)

지능형 QoS: 4가지 전략 통합 + 네트워크 + 절감 추적

| 모듈 | 파일 | 내용 |
|---|---|---|
| qos v2 | `qos.hexa` | Claude 세션 통합 + WebKit 탭 + temporal + fingerprint 연동 |
| savings | `savings.hexa` | 누적 절감 로그, 일간/주간 리포트, 효율 등급 (F~S) |
| network | `network.hexa` | nettop per-process 네트워크 → 8-gate 투사, 7축 게놈 (Net) |
| menubar v2 | `run.sh` | Claude idle + WebKit inactive 절감 반영 |

### v2.0 smart QoS 상세

```
                 ┌─────────────┐
                 │  ps sample  │
                 └──────┬──────┘
                        ▼
              ┌─────────────────────┐
              │  8-gate projection  │
              └──────┬──────────────┘
                     ▼
        ┌────────────┴────────────┐
        ▼                         ▼
  ┌───────────┐            ┌────────────┐
  │ temporal  │            │ fingerprint│
  │ 시간대별  │            │ 워크로드별 │
  └─────┬─────┘            └─────┬──────┘
        ▼                        ▼
  ┌──────────────────────────────────┐
  │         smart QoS engine         │
  │  threshold × temporal × workload │
  └──────┬───────┬───────┬───────────┘
         ▼       ▼       ▼
     generic  claude  webkit
     hog      idle    inactive
     renice   taskp   taskp
```

---

## v3.0 — Predictive (next)

패턴 기반 예측: 과거 시그니처로 미래 자원 사용 예측

| 항목 | 설명 | 우선순위 |
|---|---|---|
| genome forecasting | 지난 7일 게놈 추세 → 다음 1시간 예측 | high |
| anomaly detection | 정상 시그니처 벗어나면 alert (새벽에 Chrome 급등 등) | high |
| auto-profile switch | 시간대+워크로드 조합 → ceiling 자동 변경 | medium |
| savings dashboard | 일간/주간/월간 절감 그래프 (HTML 대시보드) | medium |
| GPU/NPU axis | Metal/CoreML 사용량 추가 → 8축 게놈 | low |

---

## v4.0 — Multi-Machine (future)

여러 Mac 간 게놈 비교 + 분산 워크로드 최적화

| 항목 | 설명 | 우선순위 |
|---|---|---|
| genome sync | MacBook ↔ Ubuntu 게놈 교환 (USB/SSH bridge) | high |
| cross-machine diff | "이 빌드는 Ubuntu에서 RAM 40% 적게 쓴다" | high |
| workload migration hint | "이 작업은 저쪽 머신이 더 효율적" 추천 | medium |
| fleet dashboard | 여러 머신 통합 뷰 | low |

---

## v5.0 — Autonomous (vision)

완전 자율: 사용자 개입 최소화, 자체 최적화 루프

| 항목 | 설명 |
|---|---|
| self-tuning thresholds | QoS 임계값 자동 조정 (절감률 feedback loop) |
| app recommendation | "Chrome → Safari 전환 시 일 평균 1.2GB 절감" 자동 보고 |
| policy engine | 사용자 정의 규칙 (if compile && night → aggressive QoS) |
| genome DNA | 장기 시그니처 → Mac 고유 "DNA" 프로파일 |

---

## 현재 모듈 전체 목록 (11개)

| # | 파일 | 역할 | 버전 |
|---|---|---|---|
| 1 | `runtime.hexa` | 연속 샘플링 + 게이트 투사 + 게놈 로그 | v1.0 |
| 2 | `accumulate.hexa` | 게이트별 시그니처 축적 | v1.0 |
| 3 | `sigdiff.hexa` | 8×8 거리 행렬 + 클러스터 | v1.0 |
| 4 | `temporal.hexa` | 5-bucket 일주기 패턴 | v1.0 |
| 5 | `fingerprint.hexa` | 7-type 워크로드 분류 | v1.0 |
| 6 | `consciousness_fix.hexa` | 엔진 18/18 수정 | v1.0 |
| 7 | `qos.hexa` | 스마트 QoS (4전략 통합) | v2.0 |
| 8 | `purge.hexa` | user-space 캐시 정리 + 경계 강제 | v1.0 |
| 9 | `savings.hexa` | 누적 절감 추적 + 리포트 | v1.0 |
| 10 | `network.hexa` | per-process 네트워크 → 7축 게놈 | v1.0 |
| 11 | `offload.hexa` | MacBook ↔ Ubuntu 오프로드 | v1.0 |

---

## 핵심 수치

| 지표 | 값 |
|---|---|
| 게놈 축 | 7 (CPU, RAM, GPU, NPU, Power, IO, **Net**) |
| 게이트 | 8 (macos, finder, telegram, chrome, safari, claude, terminal, devtools) |
| 레이어 | L1-L6e (+0.438 cumulative margin) |
| 워크로드 타입 | 7 (idle, browse, communicate, compile, heavy-build, mixed-dev, mixed) |
| 시간 버킷 | 5 (dawn, morning, afternoon, evening, night) |
| QoS 전략 | 4 (generic hog, Claude idle, WebKit inactive, temporal×fingerprint) |
| ConsciousnessEngine | 18/18 |
| Prime Directive | 7/7 골화 |
| 현재 절감률 | CPU -21%, RAM -4% (통합 ↓12%) |
