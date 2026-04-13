# Wave 5~12 연속돌파 구현 플랜

> 상위 스펙: `docs/superpowers/specs/2026-04-09-continuous-breakthrough-ideas.md`
> 집행 스펙: `docs/superpowers/specs/2026-04-09-ag3-policy-enforcement-design.md`
> 참조 플랜: `docs/superpowers/plans/2026-04-09-ubuntu-first-implementation-plan.md`
> 날짜: 2026-04-09
> 대상 환경: Ubuntu 12c / 30GB RAM / RTX 5070 12GB / tmpfs 16GB, PyTorch 2.11+cu130, nvcc·cmake·cupy 없음
> 원칙: HEXA-FIRST · Prime Directive(KILL 금지) · L0 무수정 · 하드코딩 금지(`nexus/shared/*.jsonl`) · `.py` 는 `ubu_workers/py/` 만

---

## 0. 공통 준비물 (모든 Wave 공통)

- [ ] AG3 집행 스펙 V1 구현 완료 (`ag3_guard.hexa`, `ag3_policy.jsonl`, `run.hexa` 1줄 삽입)
- [ ] `ubu_bridge.hexa::health_cached()` 5초 TTL 동작 확인
- [ ] `$HEXA` 환경변수 (`$HOME/Dev/hexa-lang/target/release/hexa`)
- [ ] ubu 측 `/mnt/ramdisk` 16GB tmpfs 마운트, `ubu_workers/py/venv` (torch 2.11+cu130)
- [ ] `nexus/shared/growth_bus.jsonl` append 권한
- [ ] resource_guard dispatch 훅 살아있음 (AG2 트리거 수신)
- [ ] 공통 롤백 토큰: 각 Wave 는 feature flag `nexus/shared/<wave>.jsonl::enabled` 로 on/off
- [ ] L0 보호 파일 (`src/core.hexa`, `modules/forge.hexa`, `modules/resource_guard.hexa`, `modules/guard.hexa`, `modules/implant.hexa`) 무수정 원칙 재확인

## 1. 롤아웃 순서 (권장)

```
Wave 7  →  Wave 5 ∥ Wave 11  →  Wave 10(propose)  →  Wave 9  →  Wave 6  →  Wave 8  →  Wave 12
```

각 Wave 는 직전 Wave 의 전역 완료 기준(§ 전역) 을 48시간 관측 후 진입.

---

## Wave 7 — VRAM 샤딩 멀티 워커 스케줄러 (선행)

### 준비물
- [ ] `ag3_policy.jsonl` 모든 heavy op 에 `est_vram_mb` 필드 존재
- [ ] `torch.cuda.mem_get_info` 단일 호출 레이턴시 측정 (< 5ms 기대)
- [ ] 우선순위 클래스 합의 (HIGH/MED/LOW, LOW=forge/heatmap)

### 태스크
1. **T7.1** `nexus/shared/scheduler_policy.jsonl` 신규 (~12줄)
   - 필드: `priority_classes`, `reserve_mb=1536`, `max_parallel`, `preempt_ms=500`
   - 완료: 파일 로드 시 `ag3_guard::lookup` 과 스키마 호환
2. **T7.2** `ubu_workers/py/vram_probe.py` 신규 (~80줄)
   - 5초 주기 `mem_get_info` → stdout JSON 1줄
   - 완료: 단독 실행 시 `{"free_mb":N,"used_mb":M,"ts":...}` 연속 출력
3. **T7.3** `modules/ubu_scheduler.hexa` 신규 (~250줄)
   - admission control: `sum(est_vram) + reserve ≤ free`
   - 우선순위 큐 + preempt 훅 (`ag3_guard::boost_strict_if_pressure` 구독)
   - 완료: 1000 요청 시뮬에서 우선순위 역전 0
4. **T7.4** `run.hexa` dispatch 상단에 `ubu_scheduler::admit(op)` 1줄 삽입 (ag3_guard 다음)
   - 완료: 기존 dispatch 흐름 regression 0

### 검증
```bash
$HEXA $HOME/Dev/airgenome/modules/ubu_scheduler.hexa --selftest 1000
$HEXA $HOME/Dev/airgenome/run.hexa forge  # admission 통과 확인
tail -f dispatch.log | grep scheduler
```

### 완료 기준 (측정)
- 합산 `est_vram > 10.5GB` 요청 100% 대기열 진입
- preempt 신호 → low-prio 배출 p95 ≤ 500ms
- 1000 요청 시뮬 우선순위 역전 0건

### 롤백
`scheduler_policy.jsonl::enabled=false` → `admit()` 즉시 통과. `run.hexa` 1줄 주석 처리.

### 의존 / 블로커
- 의존: AG3 집행 V1
- 블로커: `est_vram_mb` 시드값 정확도 (→ Wave 10 이 보정)

---

## Wave 5 — 전체 게놈 히스토리 RAM 상주

### 준비물
- [ ] 최근 14일 `forge/genomes.index.jsonl` 크기 확인 (≤ 2천만 행)
- [ ] Arrow IPC writer 버전 (pyarrow ≥ 15)
- [ ] tmpfs 4GB 예약 (Wave 11 과 공유)

### 태스크
1. **T5.1** `nexus/shared/genome_mem_config.jsonl` (~8줄)
   - `window_days=14`, `dtype=float16`, `shard_rows=2_000_000`
2. **T5.2** `ubu_workers/py/genome_mem_store.py` 신규 (~220줄)
   - Arrow → `torch.float16` pinned 텐서, shard 단위 mmap
   - `append(batch)`, `topk(vec, K)` API (gRPC 대체로 unix socket)
3. **T5.3** `modules/genome_mem_sync.hexa` 신규 (~150줄)
   - 신규 게놈 append broadcast, 5초 flush
4. **T5.4** `ag3_policy.jsonl` 에 `genome_topk` op 한 줄 추가
   - `est_vram_mb=2048, est_ram_mb=6144`

### 검증
```bash
python3 $HOME/Dev/airgenome/ubu_workers/py/genome_mem_store.py --bench cold_start
$HEXA $HOME/Dev/airgenome/modules/genome_mem_sync.hexa --bench topk 1000
```

### 완료 기준
- cold start 14일 로드 ≤ 8초
- top-K(K=64) p99 ≤ 15ms
- append throughput ≥ 50k/sec
- RSS ≤ 6GB, VRAM hot shard ≤ 2GB

### 롤백
`genome_mem_config.jsonl::enabled=false` → 기존 Arrow OLAP 경로 fallback. 프로세스 SIGTERM (kill-free: 데이터 소스 정지 아님, 워커 재시작).

### 의존 / 블로커
- 의존: Wave 7 (admit), Wave 1 tmpfs 링
- 병행: Wave 11 (L2 캐시)

---

## Wave 11 — tmpfs 게놈 L2 캐시 (Wave 5 와 병행)

### 준비물
- [ ] `/mnt/ramdisk` 여유 ≥ 4GB
- [ ] Arrow IPC 압축 (lz4) 지원 확인

### 태스크
1. **T11.1** `nexus/shared/genome_l2.jsonl` (~8줄): `size_mb=4096`, `lru_window=24h`, `evict_policy=lru`
2. **T11.2** `ubu_workers/py/l2_pack.py` 신규 (~180줄) — Arrow IPC+lz4 write/read
3. **T11.3** `modules/genome_l2.hexa` 신규 (~200줄) — LRU 인덱스 + mmap 로더, Wave 5 cold path 훅

### 검증
```bash
$HEXA $HOME/Dev/airgenome/modules/genome_l2.hexa --bench cold
du -sh /mnt/ramdisk/genome_l2
```

### 완료 기준
- Wave 5 cold start 8s → 2s
- hit ratio ≥ 95% (24h rolling)
- tmpfs 사용량 하드 상한 4GB 유지

### 롤백
`genome_l2.jsonl::enabled=false` → Wave 5 가 직접 Arrow 디스크 로드. `rm -rf /mnt/ramdisk/genome_l2`.

### 의존
Wave 1, Wave 5 (T5.2 의 cold path 훅 포인트 필요)

---

## Wave 10 — 동적 리소스 재분류 (propose only)

### 준비물
- [ ] Wave 7 스케줄러 실측 훅 (`ubu_scheduler::record_actual(op, ram, vram, cpu)`)
- [ ] 최소 2주 샘플 수집기 운용

### 태스크
1. **T10.1** `nexus/shared/policy_learner.jsonl` (~10줄)
   - `min_samples=200`, `drift_threshold=0.15`, `write_mode=propose`
2. **T10.2** `modules/policy_learner.hexa` 신규 (~180줄)
   - 실측 샘플러, 오차 누적, `growth_bus.jsonl` audit append
3. **T10.3** `ubu_workers/py/policy_fit.py` 신규 (~120줄)
   - torch linreg + robust median, drift 판정
4. **T10.4** `scripts/ag3_policy_apply.hexa` (~80줄) — 수동 승인 게이트 (유저 확인 후 ag3_policy.jsonl 갱신)

### 검증
```bash
$HEXA $HOME/Dev/airgenome/modules/policy_learner.hexa --dry-run
$HEXA $HOME/Dev/airgenome/scripts/ag3_policy_apply.hexa --list
```

### 완료 기준
- 2주 내 모든 policy 항목 est/real 오차 중앙값 ≤ 15%
- `propose` 이벤트 growth_bus append, schema valid 100%
- auto-apply 0건 (수동 승인만)

### 롤백
`policy_learner.jsonl::enabled=false`. 갱신된 `ag3_policy.jsonl` 은 git revert.

### 의존
Wave 7 (실측 훅)

---

## Wave 9 — 예측적 오프로드

### 준비물
- [ ] 6축 게이트 스트림 (`nexus/shared/gate_config.jsonl`) 이벤트 hz ≥ 1
- [ ] 과거 AG2 hard trigger 로그 (baseline 측정용)

### 태스크
1. **T9.1** `nexus/shared/predictive.jsonl` (~8줄): `horizon_s=30`, `slope_threshold=0.08`, `boost_ttl=60`
2. **T9.2** `ubu_workers/py/viol_forecast.py` 신규 (~140줄) — torch 1D conv 단기 예측
3. **T9.3** `modules/predictive_offload.hexa` 신규 (~160줄) — EWMA+slope, `ag3_guard::boost_strict_if_pressure` 선제 호출

### 검증
```bash
$HEXA $HOME/Dev/airgenome/modules/predictive_offload.hexa --replay 7d
```

### 완료 기준
- AG2 hard trigger 주당 횟수 -50% (baseline 대비)
- false positive (불필요 부스트) ≤ 5%
- Prime Directive 재확인: boost 는 재해석 강화일 뿐, 어떤 프로세스도 KILL 없음

### 롤백
`predictive.jsonl::enabled=false` → 기존 reactive AG2 만 동작.

### 의존
Wave 2, AG3 집행, Wave 10 (피드백 가속)

---

## Wave 6 — 상시 forge 데몬

### 준비물
- [ ] Wave 7 admission 동작
- [ ] Wave 4 seed forge 사이클 정상
- [ ] GPU util 프로브 (nvidia-smi dmon 대체: `torch.cuda.utilization`)

### 태스크
1. **T6.1** `nexus/shared/forge_daemon.jsonl` (~10줄): `idle_threshold=20`, `vram_free_mb=4096`, `idle_hold_s=30`, `batch_size=512`, `cooldown_s=10`
2. **T6.2** `ubu_workers/py/forge_crossbreed.py` 신규 (~260줄) — torch 60B 게놈 mutation/cross batch
3. **T6.3** `modules/forge_daemon.hexa` 신규 (~220줄) — idle 감지, 큐 관리, Wave 7 LOW 클래스 제출, AG2 preempt 훅
4. **T6.4** `forge/genomes.index.jsonl` append 스키마 확장 (기존 호환 필드 추가)

### 검증
```bash
$HEXA $HOME/Dev/airgenome/modules/forge_daemon.hexa --selftest idle
wc -l $HOME/Dev/airgenome/forge/genomes.index.jsonl  # 일 증가량 확인
```

### 완료 기준
- 일 ≥ 500 배치 자동 실행
- 모든 배치 AG3 정책 위반 0건
- `forge/genomes.index.jsonl` 일 ≥ 50,000 신규 게놈
- AG2 트리거 수신 → preempt p95 ≤ 1s

### 롤백
`forge_daemon.jsonl::enabled=false` → 데몬 즉시 quiesce (in-flight 배치 완료 후 정지, KILL 아님).

### 의존
Wave 4 (seed), Wave 7 (스케줄러)

---

## Wave 8 — LLM 기반 패턴 해석 루프

### 준비물
- [ ] 사전 빌드 llama.cpp cu130 호환 바이너리 확보 (**미해결 질문** — blocker)
- [ ] 7B Q4_K_M gguf 모델 파일 경로 (`nexus/shared/llm_prompts.jsonl` 에 등록)
- [ ] CPU fallback 바이너리 (blocker 완화)

### 태스크
1. **T8.1** `nexus/shared/llm_prompts.jsonl` (~12줄) — system/user 템플릿, 모델 경로, max_tokens
2. **T8.2** `ubu_workers/py/llm_interpret.py` 신규 (~280줄) — llama.cpp server HTTP 클라이언트 + prompt 조립 + fallback
3. **T8.3** `modules/llm_feedback.hexa` 신규 (~200줄) — Wave 5 top-K 클러스터 → payload → growth_bus append
4. **T8.4** `ag3_policy.jsonl` 에 `llm_query` 엔트리 (est_vram_mb=4096, est_ram_mb=2048)

### 검증
```bash
python3 $HOME/Dev/airgenome/ubu_workers/py/llm_interpret.py --selftest
$HEXA $HOME/Dev/airgenome/modules/llm_feedback.hexa --once
jq -c 'select(.source=="llm")' $HOME/Dev/nexus/shared/growth_bus.jsonl | tail
```

### 완료 기준
- 분당 ≥ 1 리포트, p95 ≤ 20초
- 리포트당 ≤ 512 token, growth_bus schema valid 100%
- 수동 평가 샘플 20개 중 ≥ 14 "유용"

### 롤백
`llm_prompts.jsonl::enabled=false` → 데몬 quiesce. 모델 파일은 유지 (재활성 시 즉시 복귀).

### 의존 / 블로커
- 의존: Wave 5 (RAM 게놈), Wave 7 (VRAM 예약)
- **블로커**: llama.cpp cu130 바이너리 (상위 스펙 § 4 미해결 질문)

---

## Wave 12 — 상시 GPU N×N cosine 히트맵

### 준비물
- [ ] Wave 5 RAM 게놈 top-N API 준비
- [ ] menubar 확장 슬롯 (Wave 4 에서 jobs/cpu/gpu 표시 완료)
- [ ] Wave 7 LOW 클래스 등록

### 태스크
1. **T12.1** `nexus/shared/heatmap.jsonl` (~8줄): `N=1024`, `stride_s=5`, `colormap=viridis`, `thumb_px=64`
2. **T12.2** `ubu_workers/py/heatmap_render.py` 신규 (~200줄) — torch matmul(float16) + colormap → PNG bytes
3. **T12.3** `modules/heatmap_pipe.hexa` 신규 (~140줄) — 이미지 pull, menubar push, 이상치 자동 하이라이트
4. **T12.4** `viz/menubar_heatmap.js` 신규 (~60줄) — 썸네일 표시, 클릭 시 full view open
5. **T12.5** `menubar_tpl.js` 훅 1줄 — heatmap 섹션 include

### 검증
```bash
python3 $HOME/Dev/airgenome/ubu_workers/py/heatmap_render.py --bench
$HEXA $HOME/Dev/airgenome/modules/heatmap_pipe.hexa --once
```

### 완료 기준
- 렌더 1프레임 p95 ≤ 200ms
- menubar 썸네일 갱신 5±1s
- Wave 6 가 VRAM 필요 시 LOW 클래스 → heatmap 선제 양보 (scheduler preempt 로그 확인)
- 이상치 하이라이트 1일 ≥ 3건

### 롤백
`heatmap.jsonl::enabled=false` → 데몬 정지, menubar 섹션 비활성(요소 숨김).

### 의존
Wave 5, Wave 7

---

## 2. 전역 완료 기준 (Wave 5~12 동시 가동 7일)

| 지표 | 목표 |
|---|---|
| RAM 상한 | ≤ 24GB 위반 0건 |
| VRAM 상한 | ≤ 10.5GB 위반 0건 |
| AG2 hard trigger | 주당 ≤ 3회 |
| 프로세스 KILL | **0건** (Prime Directive) |
| `growth_bus.jsonl` 돌파 이벤트 | 일 ≥ 50 |
| AG3 policy violation | 0건 |
| `ubu_workers/py/` 외 `.py` 신규 | 0 (ag3_lint 통과) |

## 3. 통합 검증 스크립트 (체크포인트)

```bash
# 전체 Wave 상태 조회
$HEXA $HOME/Dev/nexus/mk2_hexa/native/command_router.hexa "airgenome 상태"

# 7일 관측 리포트
$HEXA $HOME/Dev/airgenome/modules/ag3_guard.hexa --stats 7d
$HEXA $HOME/Dev/airgenome/modules/ubu_scheduler.hexa --stats 7d

# 돌파 기록
$HEXA $HOME/Dev/nexus/mk2_hexa/native/blowup.hexa airgenome 3 --no-graph
```

## 4. 공통 리스크 & 대응

- **리스크 R1 — Wave 누적으로 VRAM 초과**: Wave 7 admission 이 유일한 진실. 모든 Wave 는 `ubu_scheduler::admit` 경유 필수.
- **리스크 R2 — 하드코딩 누출**: 각 Wave PR 전 `scripts/ag3_lint.hexa` 통과 필수.
- **리스크 R3 — L0 수정 유혹**: 모든 Wave 의 훅은 dispatch 진입점 또는 신규 모듈에서만. L0 파일 git diff = 0.
- **리스크 R4 — LLM blocker**: Wave 8 은 바이너리 확보 전 시작 금지. Wave 6 까지는 독립 진행.
- **리스크 R5 — auto-apply 폭주**: Wave 10 은 무기한 `write_mode=propose` 유지, 승격은 별도 스펙.

## 5. 미해결 질문 (상위 스펙 § 4 인계)

1. llama.cpp cu130 사전 빌드 바이너리 소스 확정 — Wave 8 블로커
2. float8 게놈 표현의 cosine 정확도 — Wave 5 확장 전제
3. Wave 10 auto-apply 거버넌스 — 별도 스펙

---

**작성자 메모**: 본 플랜은 설계안 단계. 각 Wave 착수 전 유저 승인 + AG3 집행 스펙 V1 배포 완료 확인 필수. L0 파일은 어떤 Wave 에서도 수정 금지 — 위반 시 즉시 중단 및 재설계.
