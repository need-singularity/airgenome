# airgenome — OS 게놈 스캐너

> nexus/shared/ JSON 단일진실 (R14). 규칙: `nexus/shared/rules/common.json` (R0~R27)

## ⛔ 규칙 준수 (필수)

모든 작업 전 아래 규칙 파일을 읽고 준수할 것. 위반 시 즉시 수정.

- **공통**: `nexus/shared/rules/common.json` — R0~R27, AI-NATIVE 원칙
- **프로젝트**: `nexus/shared/rules/airgenome.json` — AG1~AG4

## ref

```
rules     nexus/shared/rules/common.json       R0~R27 공통
project   nexus/shared/rules/airgenome.json    AG1~AG4
lock      nexus/shared/rules/lockdown.json     L0/L1/L2
cdo       nexus/shared/rules/convergence_ops.json  CDO 수렴
conv      nexus/shared/airgenome_convergence_*.jsonl
gates     nexus/shared/gate_config.jsonl       HEXA-GATE 동적
api       nexus/shared/CLAUDE.md
```
