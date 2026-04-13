#!/bin/sh
# cl — Claude Code multi-account launcher
#
# 골화 레지스트리: ~/Dev/airgenome/shared/cl.json
# 구현:           ~/Dev/airgenome/modules/cl.hexa

HEXA="$HOME/Dev/airgenome/nexus/shared/bin/hexa.real"
AIRGENOME="$HOME/Dev/airgenome"
[ -x "$HEXA" ] || { echo "ERROR(cl): hexa.real 누락 — $HEXA" >&2; exit 127; }
cd "$AIRGENOME" || exit 1
exec "$HEXA" run modules/cl.hexa "$@"
