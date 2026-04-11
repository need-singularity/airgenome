# airgenome — OS 게놈 스캐너

## 명령어 (shared/config/commands.json)

| 명령 | 동작 |
|------|------|
| `todo` / `할일` | 전 프로젝트 할일 표 |
| `go` / `가자` | 모든 TODO 병렬 발사 |
| `smash` / `부셔` | 블로업 돌파 엔진 |
| `free` / `자유` | 5모듈 자율 조립 |
| `dfs` / `탐색` | DFS 재귀 깊이 탐색 |
| `list` / `목록` | 이 명령어 표 다시 출력 |

> nexus/shared/ JSON 단일진실 (R14). 규칙: `nexus/shared/rules/common.json` (R0~R27)

## ⛔ 규칙 준수 (필수)

작업 시작 전 `nexus/shared/rules/common.json` + `nexus/shared/rules/airgenome.json` 을 읽고 전 규칙 준수. 위반 시 즉시 수정.

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
