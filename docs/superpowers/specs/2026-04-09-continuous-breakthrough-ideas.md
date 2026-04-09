# Ubuntu RAM/GPU 연속 돌파 아이디어 (Wave 5~12)

> 상위 스펙 참조: `docs/superpowers/specs/2026-04-09-ubuntu-first-breakthrough-design.md`
> 정책 집행: `docs/superpowers/specs/2026-04-09-ag3-policy-enforcement-design.md`
> 날짜: 2026-04-09
> 상태: 설계안 (Wave 4 완료 후 순차 진입)

## 0. 원칙

- **HEXA-FIRST**: 모든 orchestration/IO/게이트 로직은 `.hexa`. Python(`ubu_workers/py/`)은 PyTorch/Arrow/llama.cpp 바인딩이 필요한 GPU·텐서 연산 전용.
- **Prime Directive**: 어떤 Wave도 프로세스 KILL 금지. RAM/VRAM을 채워 재해석 밀도만 올린다.
- **하드코딩 금지**: 임계값·경로·도메인·모델명은 `nexus/shared/*.jsonl`에서 로드.
- **블로커 존중**: nvcc/cmake/cupy 없음 → `torch.*` 연산자 + 사전 빌드 llama.cpp 바이너리만 사용.
- **리소스 상한**: 전체 합산 RAM ≤ 24GB, VRAM ≤ 10.5GB (호스트 여유 확보).

## 1. 리소스 예산 총괄

| Wave | RAM | VRAM | GPU 점유 모드 |
|---|---|---|---|
| 5 전체 게놈 RAM 상주 | 6GB | 2GB | 간헐 |
| 6 상시 forge 데몬 | 1GB | 3GB | 유휴 기회 |
| 7 VRAM 샤딩 스케줄러 | 0.5GB | 0 (메타) | 제어평면 |
| 8 LLM 해석 루프 | 2GB | 4GB | 간헐 |
| 9 예측적 오프로드 | 0.5GB | 0 | 제어평면 |
| 10 동적 재분류 | 0.5GB | 0 | 제어평면 |
| 11 tmpfs L2 캐시 | tmpfs 4GB | 0 | — |
| 12 상시 N×N 히트맵 | 0.5GB | 1.5GB | 5초 주기 |
| **합계** | **~15GB + tmpfs 4GB** | **~10.5GB** | — |

---

## Wave 5 — 전체 게놈 히스토리 RAM 상주

- **목표**: 최근 N일(기본 14일) 전 게놈을 단일 torch 텐서로 상주시켜 N×N 유사도를 왕복 없이 계산.
- **신규/수정 파일**:
  - `ubu_workers/py/genome_mem_store.py` (Arrow → torch.float16 pinned 텐서)
  - `modules/genome_mem_sync.hexa` (신규 게놈 append broadcast)
  - `nexus/shared/genome_mem_config.jsonl` (window_days, dtype, shard_rows)
- **리소스**: RAM 6GB (≈ 2천만 게놈 × 60B + 인덱스), VRAM 2GB (hot shard).
- **완료 기준**:
  - cold start ≤ 8초에 14일치 로드
  - 임의 게놈 1개 → top-K (K=64) 검색 p99 ≤ 15ms
  - append throughput ≥ 50k genome/sec
- **의존**: Wave 1 (tmpfs 링), Wave 2 (PyTorch 6축 게이트).
- **위험**: 30일로 확장 시 RAM 한계 접근 → shard_rows 튜닝 + float8 실험.

---

## Wave 6 — 상시 forge 데몬 (유휴 GPU 교차수분)

- **목표**: GPU util < 20% + VRAM 여유 ≥ 4GB가 N초 유지되면 자동으로 forge 교차수분 배치 실행.
- **신규/수정 파일**:
  - `modules/forge_daemon.hexa` (idle 감지 + 큐 관리, Wave 4 seed 확장)
  - `ubu_workers/py/forge_crossbreed.py` (torch 기반 60B 게놈 mutation/cross)
  - `nexus/shared/forge_daemon.jsonl` (idle_threshold, batch_size, cooldown_s)
- **리소스**: RAM 1GB, VRAM 3GB (batch 텐서).
- **완료 기준**:
  - 하루 ≥ 500 배치 자동 실행, 모든 배치 AG3 정책 위반 0건
  - AG2 트리거 시 1초 이내 preempt
  - `forge/genomes.index.jsonl`에 일 ≥ 5만 신규 게놈 등록
- **의존**: Wave 4 (forge seed), Wave 7 (스케줄러).
- **위험**: AG2와 경합 → Wave 7 preempt 훅 필수.

---

## Wave 7 — VRAM 샤딩 멀티 워커 스케줄러

- **목표**: `ag3_policy.jsonl`의 `est_vram_mb`를 기반으로 동시 실행 워커 수를 결정하는 중앙 큐.
- **신규/수정 파일**:
  - `modules/ubu_scheduler.hexa` (우선순위 큐, admission control)
  - `ubu_workers/py/vram_probe.py` (`torch.cuda.mem_get_info` 폴러)
  - `nexus/shared/scheduler_policy.jsonl` (priority_classes, reserve_mb, max_parallel)
- **리소스**: RAM 0.5GB, VRAM 0 (메타만).
- **완료 기준**:
  - 합산 est_vram > 10.5GB인 요청 자동 대기
  - 우선순위 역전 0건 (측정: 1000 요청 시뮬)
  - AG2 preempt 신호 → 500ms 내 low-prio 배출
- **의존**: AG3 정책 스펙.
- **위험**: est_vram 예측 오차 → Wave 10이 피드백 루프 제공.

---

## Wave 8 — LLM 기반 패턴 해석 루프

- **목표**: 사전 빌드 llama.cpp 7B(Q4_K_M)가 매 분 최근 이상 게놈 클러스터를 자연어 리포트로 해석 → `growth_bus.jsonl` append.
- **신규/수정 파일**:
  - `ubu_workers/py/llm_interpret.py` (llama.cpp server 호출 + prompt 조립)
  - `modules/llm_feedback.hexa` (게놈 클러스터 → prompt payload)
  - `nexus/shared/llm_prompts.jsonl` (system/user 템플릿, 하드코딩 금지)
- **리소스**: RAM 2GB, VRAM 4GB (모델 + KV 캐시).
- **완료 기준**:
  - 분당 ≥ 1 리포트 생성, p95 지연 ≤ 20초
  - 리포트당 token ≤ 512, `growth_bus.jsonl` schema valid 100%
  - 수동 평가 샘플 20개 중 ≥ 14개 "유용" 판정
- **의존**: Wave 5 (RAM 상주 게놈), Wave 7 (VRAM 예약).
- **위험**: llama.cpp 바이너리 cu130 호환 → CPU fallback path 필수.

---

## Wave 9 — 예측적 오프로드

- **목표**: 최근 violation 추세(slope)로 AG2 트리거 전에 strict 부스트를 선제 적용.
- **신규/수정 파일**:
  - `modules/predictive_offload.hexa` (EWMA + slope 계산)
  - `ubu_workers/py/viol_forecast.py` (torch 1D conv 기반 단기 예측)
  - `nexus/shared/predictive.jsonl` (horizon_s, slope_threshold, boost_ttl)
- **리소스**: RAM 0.5GB, VRAM 0.
- **완료 기준**:
  - AG2 hard trigger 발생 횟수 주당 -50%
  - false positive (불필요 부스트) ≤ 5%
- **의존**: Wave 2 (6축 게이트 스트림), AG3 집행.
- **위험**: 과도한 부스트 → Prime Directive 위배 우려. boost = 재해석 강화일 뿐, kill 금지 재확인.

---

## Wave 10 — 동적 리소스 재분류

- **목표**: `est_vram_mb`/`est_ram_mb`/`est_cpu_pct` 예측 vs 실측 오차를 수집해 `ag3_policy.jsonl`을 자동 PR.
- **신규/수정 파일**:
  - `modules/policy_learner.hexa` (실측 샘플러 + 오차 누적)
  - `ubu_workers/py/policy_fit.py` (torch linreg / robust median)
  - `nexus/shared/policy_learner.jsonl` (min_samples, drift_threshold, write_mode: propose|apply)
- **리소스**: RAM 0.5GB, VRAM 0.
- **완료 기준**:
  - 2주 내 모든 policy 항목의 est/real 오차 중앙값 ≤ 15%
  - 자동 제안 → nexus growth_bus에 audit append, 롤백 가능
- **의존**: Wave 7 (스케줄러 실측 훅).
- **위험**: auto-apply 폭주 → 초기엔 `write_mode=propose`만.

---

## Wave 11 — tmpfs 게놈 캐시 L2

- **목표**: `/mnt/ramdisk`에 hot 게놈 blob 저장 → 디스크 I/O 0, Wave 5 cold start 가속.
- **신규/수정 파일**:
  - `modules/genome_l2.hexa` (LRU + mmap 로더)
  - `ubu_workers/py/l2_pack.py` (Arrow IPC 압축 write)
  - `nexus/shared/genome_l2.jsonl` (size_mb, lru_window, evict_policy)
- **리소스**: tmpfs 4GB, RAM 오버헤드 무시 가능.
- **완료 기준**:
  - Wave 5 cold start 8s → 2s
  - hit ratio ≥ 95% (rolling 24h)
  - tmpfs 사용량 4GB 하드 상한 준수
- **의존**: Wave 1, Wave 5.
- **위험**: tmpfs OOM → Wave 9 예측적 오프로드와 coordinate.

---

## Wave 12 — 상시 GPU N×N cosine 히트맵

- **목표**: 5초 주기로 RAM 상주 게놈 중 최신 N=1024 샘플 N×N cosine → VRAM 이미지 렌더 → Mac menubar 썸네일 전송.
- **신규/수정 파일**:
  - `ubu_workers/py/heatmap_render.py` (torch matmul + colormap → PNG bytes)
  - `modules/heatmap_pipe.hexa` (이미지 pull + menubar push)
  - `viz/menubar_heatmap.js` (썸네일 표시)
  - `nexus/shared/heatmap.jsonl` (N, stride_s, colormap, thumb_px)
- **리소스**: RAM 0.5GB, VRAM 1.5GB (1024² float16 + 이미지 버퍼).
- **완료 기준**:
  - 렌더 1프레임 p95 ≤ 200ms
  - menubar 썸네일 갱신 주기 5±1s
  - GPU idle 시 Wave 6 forge에 자원 양보 (scheduler class = LOW)
- **의존**: Wave 5, Wave 7.
- **위험**: 시각화가 실제 인사이트 없이 코스메틱으로 전락 → 이상치 자동 하이라이트 포함 필수.

---

## 2. 롤아웃 순서 (제안)

1. Wave 7 (스케줄러) — 이후 모든 Wave의 admission control 기반
2. Wave 5 + Wave 11 (RAM/L2 상주)
3. Wave 10 (policy 학습 — propose only)
4. Wave 9 (예측적 오프로드)
5. Wave 6 (forge 데몬)
6. Wave 8 (LLM 해석)
7. Wave 12 (히트맵 — 마지막, 여유 자원으로)

## 3. 전역 완료 기준

- 전체 Wave 5~12 동시 가동 시:
  - RAM ≤ 24GB, VRAM ≤ 10.5GB 상한 위반 0건 / 7일
  - AG2 hard trigger 주당 ≤ 3회
  - 프로세스 KILL 0건 (Prime Directive)
  - `nexus/shared/growth_bus.jsonl`에 일 ≥ 50 돌파 이벤트 append

## 4. 미해결 질문

- llama.cpp cu130 사전 빌드 바이너리 소스 확정?
- float8 게놈 표현이 cosine 정확도에 미치는 영향 실측 필요.
- Wave 10 auto-apply 거버넌스 — 유저 승인 게이트 위치?
