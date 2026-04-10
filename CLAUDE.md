⚠️⚠️⚠️ 규칙 직접 작성 절대 금지 — shared/rules/ JSON만이 단일 진실 (R14)
⚠️ 이 파일은 nexus/shared/project-claude/airgenome.md 마스터의 심링크. 원본만 수정.
⚠️ 공통 규칙: shared/rules/common.json (R0~R27) — 반드시 준수

# airgenome — OS 게놈 스캐너

R14: nexus/shared/ JSON 단일진실, 이 파일은 참조만.

## 돌파 시스템 (AI-native 자체 창발)

⚠️ "XXX 돌파" → blowup.hexa "XXX" 3 자동 실행 (문장 전달, 키워드 추출 ❌)
⚠️ 확인 질문 없이 즉시 Bash 실행할 것
```
hexa shared/blowup/core/blowup.hexa <문장> 3 --no-graph --seeds "$(hexa shared/blowup/seed/seed_engine.hexa merge | tr '|' ',')"
```

ref:
  rules     nexus/shared/rules/common.json       R0~R27 공통
  project   nexus/shared/rules/airgenome.json    AG1~AG4
  lock      nexus/shared/rules/lockdown.json     L0/L1/L2
  cdo       nexus/shared/rules/convergence_ops.json  CDO 수렴
  conv      nexus/shared/airgenome_convergence_*.jsonl
  gates     nexus/shared/gate_config.jsonl       HEXA-GATE 동적
  api       nexus/shared/CLAUDE.md
