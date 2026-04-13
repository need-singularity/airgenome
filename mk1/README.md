# mk1 — archived monitor/gate workers (2026-04-13)

폴링 루프 기반 Mk1 모니터/게이트 레이어를 보관. Mk2 (event-driven + 단일샷 probe) 전환 시 대체됨.

## 아카이브 목적

- **이유**: 8개 워커가 5–60s 폴링 → 2,280 wakeups/hr, 상시 CPU 370%. AG6 "Mac compute zero" 규칙 정면 위반.
- **대체**: `modules/mk2/probe.hexa` (60s 단일 폴링) + `modules/mk2/gate.hexa` (Claude Code hook trigger).
- **근거**: Sutton Bitter Lesson — 모델이 똑똑해질수록 하네스는 더 단순해져야.

## 아카이브된 파일

### 모듈 6개 (mk1/)
| 파일 | 원 폴링 | 역할 |
|---|---|---|
| resource_coordinator.hexa | 10s | L3+L4+L5 통합 gate |
| resource_ceiling.hexa | 30s | 천장 모니터 + auto_fill |
| launcher_cap.hexa | 20s | AG4 launcher ≤8 |
| auto_dispatch.hexa | 5s | OFFLOAD_O1 throttle |
| ag3_loop.hexa | 5s | AG3 게놈 스캔 |
| infra_probe.hexa | 60s | 원격 상태 수집 |

### launchd plist 7개 (mk1/launchd/)
- com.airgenome.resource-coordinator.plist
- com.airgenome.resource-ceiling.plist
- com.airgenome.launcher-cap.plist
- com.airgenome.auto-dispatch.plist
- com.airgenome.ag3-loop.plist
- com.airgenome.ag3-loop-once.plist
- com.airgenome.infra-probe.plist

### L0 유지 (이 디렉토리에 없음, 제자리 단일샷 verb 로 재편 예정 — S2)
- modules/mac_compute_zero.hexa
- modules/load_balancer.hexa
- modules/ag3_menubar_feed.hexa
- scripts/com.airgenome.mac-compute-zero.plist
- scripts/com.airgenome.load-balancer.plist

## bitter-gate audit (S0, 2026-04-13)

```
audited: 33 rules
active:  1
dormant: 0
insufficient: 32 (min_samples=50, 현재=35)
```

샘플 부족으로 dormant 판정 보류. 재평가 시점: lint_log+gc_log 합계 ≥50 도달 후.

## 롤백 절차

1. `cp mk1/launchd/*.plist ~/Library/LaunchAgents/`
2. `cp mk1/*.hexa modules/`
3. 각 label 에 대해 `launchctl bootstrap gui/501 ~/Library/LaunchAgents/com.airgenome.<label>.plist`
4. Mk2 plists 언로드: `launchctl bootout gui/501/com.airgenome.mk2-probe` (등)
5. `airgenome/.claude/settings.json` 에서 Mk2 hook 3줄 `git revert`
6. `hexa run ~/Dev/nexus/shared/lockdown/l0_guard.hexa verify` → 47 PASS 재확인

## 참고
- plan: `modules/mk2/plan.json`
- 원본 플랜 md: `~/.claude/plans/ticklish-drifting-beacon.md`
- 하네스 원칙: `nexus/shared/harness/principles.jsonl`
