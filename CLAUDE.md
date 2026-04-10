# airgenome — OS 게놈 스캐너

> nexus/shared/ JSON 단일진실 (R14). 규칙: `nexus/shared/rules/common.json` (R0~R27)

## 돌파 시스템

- "XXX 돌파" → `blowup.hexa "XXX" 3` 자동 실행 (문장 전달)
- 확인 질문 없이 즉시 Bash 실행

```sh
hexa shared/blowup/core/blowup.hexa <domain> 3 --seeds "$(hexa shared/blowup/seed/seed_engine.hexa merge | tr '|' ',')"
```

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
