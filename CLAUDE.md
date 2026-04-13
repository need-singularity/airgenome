# airgenome — OS 게놈 스캐너

commands: shared/config/commands.json — autonomous 블록으로 Claude Code가 작업 중 smash/free/todo/go/keep 자율 판단·실행
rules: ~/Dev/nexus/shared/rules/common.json (R0~R32) + ~/Dev/nexus/shared/rules/airgenome.json (AG1~AG9)
L0 Guard: `hexa ~/Dev/nexus/shared/lockdown/l0_guard.hexa <verify|sync|merge|status>`

ref:
  rules     ~/Dev/nexus/shared/rules/common.json        R0~R32
  project   ~/Dev/nexus/shared/rules/airgenome.json     AG1~AG9
  lock      ~/Dev/nexus/shared/lockdown/lockdown.json   L0/L1/L2
  cdo       ~/Dev/nexus/shared/rules/convergence_ops.json  CDO 수렴
  conv      nexus/shared/airgenome_convergence_*.jsonl
  gates     nexus/shared/gate_config.jsonl              HEXA-GATE 동적
  api       ~/Dev/nexus/shared/CLAUDE.md
