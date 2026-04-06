#!/bin/sh
cd ~/Dev/airgenome
OUTPUT=$(~/Dev/hexa-lang/hexa run modules/cl.hexa "$@" 2>&1)
LAUNCH_DIR=$(echo "$OUTPUT" | grep '^LAUNCH:' | sed 's/^LAUNCH://')
# Print everything except the LAUNCH marker
echo "$OUTPUT" | grep -v '^LAUNCH:'
# If LAUNCH marker found, exec claude with that config dir
if [ -n "$LAUNCH_DIR" ]; then
    CLAUDE_BIN=$(which claude 2>/dev/null || echo ~/.local/bin/claude)
    exec env CLAUDE_CONFIG_DIR="$LAUNCH_DIR" "$CLAUDE_BIN"
fi
