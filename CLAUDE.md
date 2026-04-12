# airgenome — OS 게놈 스캐너

commands: shared/config/commands.json — autonomous 블록으로 Claude Code가 작업 중 smash/free/todo/go/keep 자율 판단·실행
rules: nexus/shared/rules/common.json (R0~R27) + nexus/shared/rules/airgenome.json (AG1~AG4)
L0 Guard: `hexa ~/Dev/nexus/shared/lockdown/l0_guard.hexa <verify|sync|merge|status>`

ref:
  rules     nexus/shared/rules/common.json       R0~R27
  project   nexus/shared/rules/airgenome.json    AG1~AG4
  lock      nexus/shared/rules/lockdown.json     L0/L1/L2
  cdo       nexus/shared/rules/convergence_ops.json  CDO 수렴
  conv      nexus/shared/airgenome_convergence_*.jsonl
  gates     nexus/shared/gate_config.jsonl       HEXA-GATE 동적
  api       nexus/shared/CLAUDE.md
