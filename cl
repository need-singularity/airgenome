#!/bin/sh
cd ~/Dev/airgenome
OUTPUT=$(~/Dev/hexa-lang/hexa run modules/cl.hexa "$@" 2>&1)
LAUNCH_DIR=$(echo "$OUTPUT" | grep '^LAUNCH:' | sed 's/^LAUNCH://')
# Print everything except the LAUNCH marker
echo "$OUTPUT" | grep -v '^LAUNCH:'
# If LAUNCH marker found, start login shell with CLAUDE_CONFIG_DIR set
if [ -n "$LAUNCH_DIR" ]; then
    export CLAUDE_CONFIG_DIR="$LAUNCH_DIR"
    exec /bin/zsh -l -c "claude"
fi
