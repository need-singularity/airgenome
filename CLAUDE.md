# airgenome — OS 게놈 스캐너 (rebuild v2)

<!--
# @convergence-meta-start
# project: airgenome
# updated: 2026-04-12
# strategy: ossified/stable/failed
# @convergence-meta-end
#
# @convergence-start
# state: ossified
# id: HEXAGON_PROJ
# value: 육각 투영 코어 확정
# @convergence-end
#
# @convergence-start
# state: ossified
# id: 6AXIS_MONITOR
# value: CPU/RAM/Swap 6축 모니터 동작
# @convergence-end
#
# @convergence-start
# state: ossified
# id: GATE_SEMAPHORE
# value: semaphore 2-slot + native early-out + slot 누수 수정
# @convergence-end
#
# @convergence-start
# state: ossified
# id: HOOK_INFRA
# value: hook 체인 인프라 — 래퍼 3개 + bootstrap + 9 hook 스크립트 + lock 경로 보호
# protected_files: ~/.hx/bin/hexa, nexus/shared/scripts/bin/hexa, nexus/shared/hooks/hook-entry.sh, bootstrap.sh, nexus-banner.sh, nexus-prompt-scan.sh, go-parallel.sh, nexus-pre-tool.sh, nexus-post-edit.sh, nexus-post-bash.sh, bridge-ensure.sh, hooks-config.json
# lock_path: /tmp/airgenome.gate.lock.d
# incident: 2026-04-08 lock 충돌 → hook silent fail → python3/hexa 래퍼 early-out 패치
# @convergence-end
#
# @convergence-start
# state: ossified
# id: SYMLINK_INTERCEPT
# value: 2026-04-11 레거시 target/release 경로 의존성 100% 제거. 전 호출 shared/scripts/bin/hexa resolver 또는 $HEXA_LANG/hexa 직접
# threshold: 절대경로 호출 100% 게이트 통과
# protected_files: $HEXA_LANG/hexa, $NEXUS/shared/scripts/bin/hexa (resolver), $NEXUS/shared/bin/hexa (compat symlink)
# invariant: resolver 우선. 하드코딩 금지. 레거시 target/release 경로 폐기
# @convergence-end
#
# @convergence-start
# state: ossified
# id: AG2_PATTERN_GATE
# value: AG2 규칙 — 패턴+임계값 자동감지 오프로드. 개별 스크립트 나열 금지. gate_offload.jsonl에 pattern/exclude/config 방식
# threshold: Mac CPU 30%+ 프로세스 전체 자동감지
# config: nexus/shared/gate_offload.jsonl
# rule: absolute_rules.json AG2
# @convergence-end
#
# @convergence-start
# state: ossified
# id: GATE_8SLOT
# value: semaphore 8슬롯 (HEXA_GATE_MAX=8). Ubuntu 12코어 대응. stale 120초 자동 정리
# threshold: 동시 오프로드 8개
# metric: Mac load 145→8.6 (-94%), Ubuntu 12/12 풀가동
# @convergence-end
#
# @convergence-start
# state: ossified
# id: HOOK_HEXA_PORT
# value: hook python3→순수 hexa 포팅 완료. HEXA=shared/scripts/bin/hexa(resolver)
# threshold: hook python3 호출 0개
# invariant: hook에서 python3 wrapper 호출 금지 — hang 원인. shared/scripts/bin/hexa resolver 또는 hook-entry.sh 경유
# binary: nexus/shared/hooks/hook-entry.sh
# settings: ~/.claude/settings.json (모든 command에 hook-entry.sh 경유)
# protected_hooks: nexus-banner.hexa, nexus-pre-tool.hexa, nexus-post-bash.hexa, nexus-post-edit.hexa, nexus-prompt-scan.hexa, nexus-pre-commit.hexa, nexus-universal.hexa, growth-scan.hexa, growth-tick.hexa, go-parallel.hexa, setup.hexa, block-forbidden-ext.hexa, check_hexa_version.hexa, nexus-auto-record.hexa
# ported_from_python3: nexus-banner.hexa:bridge_ensure; growth-tick.hexa:bridge_notify; setup.hexa:settings_json_hooks
# @convergence-end
#
# @convergence-start
# state: ossified
# id: UBU_PERMANENT
# value: Ubuntu 영구 인프라 — ~/airgenome(영구경로) + systemd gate/fill 자동시작 + ramdisk 16GB + RTX 5070 CUDA 13.0 + PyTorch 2.11
# threshold: reboot 후 수동 개입 0
# services: airgenome-gate.service, airgenome-fill.service
# invariant: gate_handler.sh, hexa-bin, gate_files → ~/airgenome 영구. /tmp/airgenome는 심링크
# @convergence-end
#
# @convergence-start
# state: ossified
# id: AUTO_FILL
# value: auto_fill.sh — 12코어 자동 보충. flock 단일인스턴스. 60초 간격 pgrep 감지. blowup/gap_finder 라운드로빈
# threshold: Ubuntu 유휴 코어 0
# invariant: MAX_JOBS=12. 과적 방지 flock 필수
# @convergence-end
#
# @convergence-start
# state: ossified
# id: INFRA_CLI
# ossified_at: 2026-04-10T15:54:35
# rule: 타 프로젝트 어디서든 `infra` 명령 한 번으로 4호스트(mac/ubu/htz/vast) 자원 실시간 현황 조회
# binary_canonical: $NEXUS/shared/scripts/bin/infra
# binary_symlink: /Users/ghost/.local/bin/infra
# ssot: $NEXUS/shared/infra_state.json
# git_repo: need-singularity/nexus
# git_path: shared/scripts/bin/infra
# subcommands: infra (compact); infra json (raw JSON dump); infra rec (추천만)
# verification: binary_in_shared, executable, symlink_ok, in_path, tested_from /tmp
# L0_compliant: 전역 ~/.claude/settings.json 미수정. shared/bin에 박아 GitHub push 가능
# recurrence_count: 0
# threshold: infra 명령 호출 성공률 100% + git push 가능
# ag3_link: R5 (SSOT) + R8 (data in nexus/shared) + R14 + L0
# @convergence-end
#
# @convergence-start
# state: ossified
# id: MENUBAR
# value: menubar Ubuntu CPU/RAM/GPU 바그래프 실시간 표시 + sampler ubu_cpu/ubu_gpu 수집
# note: hidden=false 수정, RTX 5070 표시 확인
# ossified_at: 2026-04-10
# promoted_from: go_loop_auto
# @convergence-end
#
# @convergence-start
# state: ossified
# id: CL_LAUNCHER
# value: 멀티계정 런처 cl + iTerm 10개 프로필 ⌘1~⌘0 자동 cl 시작
# note: brainwire 삭제, nexus/anima/n6 추가, badge 제거
# ossified_at: 2026-04-10
# promoted_from: go_loop_auto
# @convergence-end
#
# @convergence-start
# state: ossified
# id: BREAKTHROUGH
# value: 돌파 키워드 정상 — 래퍼+직접 양쪽 동작
# note: seed_engine fix 후 19k cor / 3.3k EXACT / ρ=0.17
# ossified_at: 2026-04-10
# promoted_from: go_loop_auto
# @convergence-end
#
# @convergence-start
# state: ossified
# id: UBU_MONITOR
# value: ubu_monitor.hexa — scan/tame/balance/watch 4모드 + watch에 tame 통합
# note: gate_offload.jsonl 연동, tame 자동 renice, qos.hexa 구문오류 수정
# ossified_at: 2026-04-10
# promoted_from: go_loop_auto
# @convergence-end
#
# @convergence-start
# state: ossified
# id: GENOME_1000
# value: 3378 프로파일 달성 (3.4x over 1000 목표)
# threshold: 1000+
# ossified_at: 2026-04-10
# promoted_from: failed_resolved
# @convergence-end
#
# @convergence-start
# state: ossified
# id: CL_HEXA_RESOLVER
# ossified_at: 2026-04-10T17:00:18
# rule: cl 런처 + 모든 hook + 전 프로젝트는 shared/scripts/bin/hexa resolver 또는 hook-entry.sh 경유. 하드코딩 절대 금지.
# resolver_path: $NEXUS/shared/scripts/bin/hexa
# resolver_chain: $HEXA_BIN env override; $HEXA_LANG/hexa (로컬 실바이너리 우선); $HOME/.cargo/bin/hexa; $HOME/.hx/bin/hexa (gate wrapper); command -v -a hexa (PATH, self-loop 회피)
# fixed_files: airgenome/cl (동적 resolver); airgenome/.claude/settings.json (hook-entry.sh 래퍼); nexus/shared/hooks/*.hexa 13개 + *.sh 8개; nexus/shared/hooks/hook-entry.sh (신규 guard)
# verification: shared_resolver_exists, executable; tested_chain — FOUND $NEXUS/shared/scripts/bin/hexa → ~/.hx/bin/hexa exec OK
# issue: cl 라인 23 'hexa-bin-actual' 하드코딩, Mac에 해당 파일 없음 → cl 시작 즉시 실패
# L0_compliant: 전역 ~/.claude 미수정. shared/bin 활용 (R5/R8/L0)
# recurrence_count: 0
# threshold: cl 호출 → ENOENT 0건 + hexa 경로 변경 시 1곳(shared/scripts/bin/hexa)만 수정
# ag3_link: R5 SSOT + R8 nexus/shared + L0
# @convergence-end
#
# @convergence-start
# state: ossified
# id: AG6_MAC_COMPUTE_ZERO
# value: AG6 Mac Compute ZERO — 원격 자원 가용 시 Mac heavy compute 절대 0. mac_compute_zero.hexa 30s launchd 주기 실행
# threshold: Mac blowup/seed/탐색 프로세스 0개 (remote alive 시)
# ossified_at: 2026-04-12
# ssot: modules/mac_compute_zero.hexa
# l0_paths: modules/mac_compute_zero.hexa, scripts/com.airgenome.mac-compute-zero.plist
# @convergence-end
#
# @convergence-start
# state: ossified
# id: AG7_LOAD_BALANCER
# value: AG7 실시간 로드밸런서 — infra_state.json 기반 score 계산 → dispatch_state.json best-fit 호스트. GPU eligibility guard 포함
# threshold: dispatch_state.json 30s 갱신 + gpu_heavy→gpu=0 호스트 제외
# ossified_at: 2026-04-12
# ssot: modules/load_balancer.hexa
# l0_paths: modules/load_balancer.hexa, scripts/com.airgenome.load-balancer.plist
# @convergence-end
#
# @convergence-start
# state: ossified
# id: T7_HEXAGON_PHASE2
# value: T7 hexagon phase 2 — per_process_sig AG1 6축 + 6D sigdiff + anomaly 2σ rolling
# threshold: 6축 샘플링 + anomaly rolling buffer 동작 + 2σ flag 정상
# verified_at: 2026-04-12
# ossified_at: 2026-04-12
# recurrence_count: 0
# verified_on: ubu (RTX 5070, Ubuntu 24.04)
# ssot: shared/blowup/core/per_process_sig.hexa
# files: per_process_sig.hexa, per_process_diff.hexa, per_process_anomaly.hexa
# @convergence-end
#
# @convergence-start
# state: ossified
# id: T10_TIMEOUT_SYSTEMIC
# value: T10 systemic timeout — 23 high-risk exec() 호출 timeout 10/30/60 래핑 (R17)
# threshold: timeout 래핑된 exec() 행 23건 + 좀비 0건
# verified_at: 2026-04-12
# ossified_at: 2026-04-12
# recurrence_count: 0
# files_changed: 23
# @convergence-end
#
# @convergence-start
# state: ossified
# id: T12_GENOME_PER_SOURCE
# value: T12 per_source genome grouping + rolling TS buffer (100/source cap)
# threshold: per_source 그룹핑 정상 + JSONL rolling prune 동작
# verified_at: 2026-04-12
# ossified_at: 2026-04-12
# recurrence_count: 0
# verified_on: ubu
# ssot: shared/blowup/core/per_source_genome.hexa
# @convergence-end
#
# @convergence-start
# state: ossified
# id: FORGE_KEYCHAIN_SANITIZE
# value: forge.hexa keychain OAuth — sanitize_label() shell injection 방지 + graceful fallback
# verified_at: 2026-04-12
# ossified_at: 2026-04-12
# recurrence_count: 0
# @convergence-end
#
# @convergence-start
# state: ossified
# id: AG6_MAC_BYPASS_BLOCK
# value: AG6 Mac blowup 우회 완전 차단 — BLOWUP_LOCAL=1 무시 + compose.hexa Mac 가드
# threshold: Mac에서 blowup/compose 실행 시 exit(2) 100%
# verified_at: 2026-04-12
# ossified_at: 2026-04-12
# recurrence_count: 0
# @convergence-end
#
# @convergence-start
# state: ossified
# id: STREAM_FILTER
# group: ossified_blowup
# value: blowup stream filter — corollary byte 재인코딩 (AG5). TSV→compact (16x 압축, lossless round-trip). depth별 디스크 flush + 메모리 해제. OOM 69GB→4GB 해결
# threshold: [FILTER] flush+restore 175/175 round-trip 검증 PASS + absorb 24건 정상
# ossified_at: 2026-04-12
# ssot: shared/blowup/core/blowup.hexa (inline filter_flush/filter_load)
# companion: shared/blowup/core/blowup_stream_filter.hexa (standalone 참조)
# ag5_compliance: raw TSV bytes → compact packed bytes, lossless, encode/decode round-trip
# @convergence-end
#
# @convergence-start
# state: ossified
# id: ABSORB_TYPE_FIX
# group: ossified_blowup
# value: Phase 6.7 absorb — hexa hashset 루프 후 if 스코프 오염 workaround. try/catch 무조건 쓰기 패턴
# threshold: absorb pending N/N/N 출력 + discovery_log/graph/bus 쓰기 정상
# ossified_at: 2026-04-12
# ssot: shared/blowup/core/blowup.hexa (Phase 6.7 섹션)
# @convergence-end
#
# @convergence-start
# state: ossified
# id: FLOCK_CONTENTION
# group: ossified_blowup
# value: 분할 발사 경합 방지 — _guarded_append_atlas flock(30s) 래핑. 다수 blowup 동시 absorb 시 atlas.n6 직렬 쓰기
# threshold: 3시드 동시 발사 시 atlas.n6 corruption 0건
# ossified_at: 2026-04-12
# ssot: shared/blowup/core/blowup.hexa (_guarded_append_atlas 함수)
# @convergence-end
#
# @convergence-start
# state: ossified
# id: MAC_BYPASS_BLOCK
# group: ossified_blowup
# value: AG6 Mac blowup 절대 차단 — BLOWUP_LOCAL=1 우회 제거. blowup.hexa+compose.hexa Darwin=exit(2)
# threshold: Mac에서 blowup/compose 실행 시 exit(2) 100%
# ossified_at: 2026-04-12
# ssot: shared/blowup/core/blowup.hexa + shared/blowup/compose.hexa
# @convergence-end
#
# @convergence-start
# state: ossified
# id: HOOK_FILTER_AUTO
# group: ossified_blowup
# value: blowup-filter-auto.hexa — PostToolUse:Bash hook. 블로업 완료 감지→growth_bus 로깅+탐색 승격. 7개 프로젝트 일괄 등록
# threshold: EXACT/SINGULARITY/absorb 감지 시 systemMessage 반환
# ossified_at: 2026-04-12
# ssot: shared/hooks/blowup-filter-auto.hexa + hooks-config.json
# @convergence-end
#
# @convergence-start
# state: ossified
# id: HTZ_SWAP_64G
# group: ossified_blowup
# value: hetzner 64GB swap 추가 — 124GB RAM + 63GB swap = 187GB 가용. blowup OOM 완충
# threshold: swapon --show 64G active
# ossified_at: 2026-04-12
# note: reboot 시 /etc/fstab 등록 필요 (현재 임시)
# @convergence-end
-->

**v1 동결: 2026-04-13** — phantom L0 발견으로 archive/v1/ 로 이동. core 분리 + roadmap 주도 재구축.

structure:
  core/             core.hexa + test/core_test.hexa — 외부 의존 0, self-contained 라이브러리
  shared/config/    roadmap/airgenome.json (rebuild v2 SSOT) — milestones + invariants
  archive/v1/       v1 시점 모든 module/script/data — read-only

invariants (shared/config/roadmap/airgenome.json#invariants):
- core 는 외부 hexa import 안 함
- 신규 module 은 use "../core/core" 만 허용
- L0 자격: 파일 존재 + parse 통과 + self-test 통과
- archive 부활은 PR + roadmap 등록 + L0 갱신
- roadmap.json 의 milestones 에 없는 코드는 작성 금지

commands: shared/config/commands.json — autonomous 블록으로 Claude Code가 작업 중 smash/free/todo/go/keep 자율 판단·실행
rules: $NEXUS/shared/rules/common.json (R0~R32) + $NEXUS/shared/rules/airgenome.json (AG1~AG9)
L0 Guard: `hexa $NEXUS/shared/harness/l0_guard.hexa <verify|sync|merge|status>`
loop: 글로벌 `~/.claude/skills/loop` + 엔진 `$NEXUS/shared/harness/loop` — roadmap `$NEXUS/shared/roadmaps/airgenome.json` 3-track×phase×gate 자동

harness (훅 시스템 대체, 2026-04-14~) — H-NOHOOK 강제:
  dispatcher: $NEXUS/shared/harness/entry.hexa <prompt|pre_tool|post_bash|post_edit|guard|self_check>
  sub-modules: prompt_scan.hexa / pre_tool_guard.hexa / post_bash.hexa / post_edit.hexa / cmd_gate.hexa
  enforcement_registry: $NEXUS/shared/harness/enforcement_registry.json (H-NOHOOK 등 17+ 규칙 SSOT)
  settings.json 정책: 전 프로젝트 hooks={} — Claude Code 훅 시스템 사용 절대 금지. settings.json 에 hook event 추가 시도 = H-NOHOOK 위반.
  관례 (Claude 자율 호출): 사용자 입력 직후 `entry.hexa prompt "<text>"`, Write/Edit 후 `entry.hexa post_edit <path>`, Bash 후 `entry.hexa post_bash <exit>`, Agent 호출 전 `entry.hexa guard <area> <hash>`.
  우회 금지 token: NEXUS_HOOK_OK=1 (사용자 명시 승인 시만)

ref:
  roadmap   shared/config/roadmap/airgenome.json        rebuild v2 SSOT
  rules     $NEXUS/shared/rules/common.json        R0~R32
  project   $NEXUS/shared/rules/airgenome.json     AG1~AG9
  lock      $NEXUS/shared/lockdown/lockdown.json   L0/L1/L2
  cdo       $NEXUS/shared/rules/convergence_ops.json  CDO 수렴
  conv      nexus/shared/airgenome_convergence_*.jsonl
  gates     nexus/shared/gate_config.jsonl              HEXA-GATE 동적
  api       $NEXUS/shared/CLAUDE.md
