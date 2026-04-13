# airgenome

OS 게놈 스캐너 — Mac/원격 vitals 를 6축 hexagon (60바이트) 게놈으로 투사, 패턴 누적, anomaly 검출.

**Status**: rebuild v2 진행 중 (2026-04-13 동결). 자세한 내용은 [`shared/config/roadmap/airgenome.json`](shared/config/roadmap/airgenome.json).

## Layout

```
core/              # 분리된 라이브러리 — Vitals, sample, assess, AdaptiveThrottle
  core.hexa
  test/core_test.hexa
shared/config/
  roadmap/airgenome.json   # rebuild v2 SSOT (milestones, invariants)
  ...
archive/v1/        # v1 시점 모든 코드/데이터 (read-only)
nexus/             # cross-project SSOT (별도 프로젝트)
CLAUDE.md          # 프로젝트 인스트럭션 (Claude Code)
cl                 # claude wrapper
```

## Run core self-test

```bash
hexa run core/test/core_test.hexa
```

## L0 verify

```bash
hexa ~/Dev/nexus/shared/lockdown/l0_guard.hexa verify
```

## Roadmap

```bash
jq '.milestones | map({id, title, status})' shared/config/roadmap/airgenome.json
```

다음 작업 (unblocked):

```bash
jq '.milestones | map(select(.status == "todo" and ((.deps | length) == 0)))' shared/config/roadmap/airgenome.json
```

## Archive

v1 의 모든 코드는 `archive/v1/` 에 동결. 부활 절차는 [`archive/v1/README.md`](archive/v1/README.md).
