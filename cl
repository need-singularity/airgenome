#!/bin/sh
# cl — Claude Code multi-account launcher
#
# 골화 레지스트리: ~/Dev/airgenome/shared/cl.json

echo "[cl] 시작…" >&2
HEXA="$HOME/Dev/airgenome/nexus/shared/bin/hexa.real"
AIRGENOME="$HOME/Dev/airgenome"
if [ ! -x "$HEXA" ]; then
    echo "ERROR(cl): hexa.real 누락 — $HEXA" >&2
    exit 127
fi
cd "$AIRGENOME" || exit 1
echo "[cl] hexa.real run modules/cl.hexa $@" >&2
exec "$HEXA" run modules/cl.hexa "$@"
