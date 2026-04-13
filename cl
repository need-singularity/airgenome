#!/bin/sh
# cl — Claude Code multi-account launcher
echo "[cl:1] 시작 pid=$$" >&2
HEXA="$HOME/Dev/airgenome/nexus/shared/bin/hexa.real"
AIRGENOME="$HOME/Dev/airgenome"
echo "[cl:2] HEXA=$HEXA" >&2
test -x "$HEXA" || { echo "ERROR: hexa.real 누락" >&2; exit 127; }
cd "$AIRGENOME" || { echo "ERROR: cd $AIRGENOME" >&2; exit 1; }
echo "[cl:3] cwd=$(pwd) args=$@" >&2
echo "[cl:4] run 시작" >&2
"$HEXA" run modules/cl.hexa "$@"
ECODE=$?
echo "[cl:5] 종료 EC=$ECODE" >&2
exit $ECODE
