#!/bin/sh
# cl — Claude Code multi-account launcher (hexa AI-native)
#
# 골화 레지스트리: ~/Dev/airgenome/shared/cl.json
# 구현:           ~/Dev/airgenome/modules/cl_runner.hexa (launcher wrapper)
#                 ~/Dev/airgenome/modules/cl.hexa (subcommand logic)
#
# 절대규칙:
#   1. zsh 배열 1-based (bash 흔적 — hexa runner는 0-based)
#   2. 세션 종료 후 `claude -p ok --max-turns 1` → 키체인 갱신
#   3. accounts.json config_dir trailing slash 금지
#   4. usage: $HEXA modules/usage.hexa -- one NAME
#   5. 쿨다운 초기화: echo '{}' > ~/.airgenome/refresh-cooldown.json
#
# 이 shell 은 hexa runner 로의 thin dispatcher 만. HX4 준수 진행 상태.

HEXA="$HOME/Dev/airgenome/nexus/shared/bin/hexa.real"
RUNNER="$HOME/Dev/airgenome/modules/cl_runner.hexa"
if [ ! -x "$HEXA" ]; then
    echo "ERROR(cl): hexa.real 누락 — $HEXA" >&2
    exit 127
fi
exec "$HEXA" run "$RUNNER" "$@"
