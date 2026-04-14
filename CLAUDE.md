# airgenome — OS 게놈 스캐너 (rebuild v2)

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
