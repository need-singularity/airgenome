#!/bin/sh
# cl — Claude Code multi-account launcher
#
# 골화 레지스트리: ~/Dev/airgenome/shared/cl.json
# 구현:           ~/Dev/airgenome/modules/cl.hexa (subcommand + launch logic)
#                 ~/Dev/airgenome/modules/cl_runner.hexa (WIP — ai-native wrapper, 검증 중)
#
# 절대규칙:
#   1. zsh 배열 1-based
#   2. 세션 종료 후 `claude -p ok --max-turns 1` → 키체인 갱신
#   3. accounts.json config_dir trailing slash 금지
#   4. usage: $HEXA modules/usage.hexa -- one NAME
#   5. 쿨다운 초기화: echo '{}' > ~/.airgenome/refresh-cooldown.json

HEXA="$HOME/Dev/airgenome/nexus/shared/bin/hexa.real"
AIRGENOME="$HOME/Dev/airgenome"
if [ ! -x "$HEXA" ]; then
    echo "ERROR(cl): hexa.real 누락 — $HEXA" >&2
    exit 127
fi
cd "$AIRGENOME"
exec "$HEXA" run modules/cl.hexa "$@"
