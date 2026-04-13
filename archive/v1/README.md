# archive/v1 — airgenome v1 동결

**동결일**: 2026-04-13
**사유**: rebuild v2 — core 분리 + phantom L0 차단 + roadmap 주도 재설계

## 무엇이 들어 있는가

v1 시점의 모든 모듈/스크립트/데이터/문서.

| 분류 | 경로 (archive/v1 기준) | 비고 |
|---|---|---|
| hexa 모듈 | `modules/`, `mk2_hexa/` | 45+ 파일 |
| LaunchAgent plist | `scripts/com.airgenome.*.plist` | rebuild 시점에 전부 unload |
| 데이터 | `forge/` | genomes.pack, genomes.index.jsonl, task_results.jsonl 등 |
| Mac 앱 | `Airgenome.app/` | applet 패키지 |
| 문서 | `roadmap.md`, `plan.md`, `CHANGELOG.md`, `HEXA_BLOCKERS.md`, `README.md`(루트), `docs/` | |
| 기타 | `api/`, `gate/`, `viz/`, `tests/`, `ubu_workers/`, `void/`, `target/`, `logs/` | |
| 루트 잡파일 | `_ttest.hexa`, `cl_mk1`, `dispatch.log`, `genomes.events.jsonl`, `genomes.log`, `hexa.toml`, `launcher_cap.hexa`, `menubar.hexa`, `menubar_tpl.js`, `offload.log`, `prime_directive.json`, `profiles.json`, `run.hexa`, `sampler.hexa`, `settings.hexa`, `consciousness_engine_status.json` | |

## 왜 동결했는가

L0 47 PASS / 0 FAIL 상태에서도 다수 phantom 파일 발견:
- `src/core.hexa` — 모든 hexa 바이너리에서 파싱 실패 (rebuild 직전 PR #31 에서 수정 후 `core/core.hexa` 로 이동)
- `modules/load_balancer.hexa` — `try`/`catch` 미지원 구문 + `Abort trap: 6` 반복
- 기타 다수 모듈 — 검증된 적 없음

**근본 원인**: L0 guard 가 "파일 존재" 만 검증, "파싱/실행" 미검증. dead code 누적.

## 부활 절차

archive 의 파일을 활성으로 되돌리려면:

1. `git mv archive/v1/<path> <new_path>` — 적절한 새 위치
2. 해당 .hexa 가 `hexa run <file>` 으로 파싱 통과 확인
3. core 만 의존하도록 리팩터 (`use "../core/core"`)
4. `shared/config/roadmap/airgenome.json` 의 milestones 또는 items 에 등록
5. `~/Dev/nexus/shared/lockdown/lockdown.json` 의 `projects.airgenome.L0` 에 추가 (PR 필요)
6. CODEOWNERS 자동 sync — `hexa ~/Dev/nexus/shared/scripts/sync_codeowners.hexa`
7. PR + `hexa ~/Dev/nexus/shared/lockdown/l0_guard.hexa merge <PR#>`

부활 없이 영구 archive 로 두는 것이 기본값.

## 참조

- 직전 활성 커밋: `8b09fa3 Merge pull request #31 from need-singularity/fix/core-hexa-syntax`
- 이전 roadmap: `archive/v1/roadmap.md` (v1.0–v5.0 vision)
- ROI SOT (history): `shared/config/roi/airgenome.json` (rebuild 후에도 참조 가능)
