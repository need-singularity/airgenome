#!/bin/zsh -l
ORIG_DIR="$(pwd)"
cd ~/Dev/airgenome
OUTPUT=$(~/Dev/hexa-lang/hexa run modules/cl.hexa "$@" 2>&1)
LAUNCH_DIR=$(echo "$OUTPUT" | grep '^LAUNCH:' | sed 's/^LAUNCH://')
cd "$ORIG_DIR"
# Print everything except the LAUNCH marker
echo "$OUTPUT" | grep -v '^LAUNCH:'
# If LAUNCH marker found, exec claude in caller's directory
if [ -n "$LAUNCH_DIR" ]; then
    export CLAUDE_CONFIG_DIR="$LAUNCH_DIR"
    exec claude
else
    echo ""
    echo "[cl] claude 실행 실패 — LAUNCH 마커 없음"
    echo "[cl] 아무 키나 누르면 종료..."
    read -r _
fi
