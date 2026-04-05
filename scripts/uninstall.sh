#!/usr/bin/env bash
# airgenome — clean uninstall.
#
#   curl -fsSL https://raw.githubusercontent.com/need-singularity/airgenome/main/scripts/uninstall.sh | bash
#
# Unloads the LaunchAgent, removes the plist, and uninstalls the binary.
# The ${HOME}/.airgenome data directory is left in place (delete manually
# if you want to wipe recorded vitals).

set -euo pipefail

AGENT_LABEL="com.airgenome.daemon"
AGENT_PLIST="${HOME}/Library/LaunchAgents/${AGENT_LABEL}.plist"
DATA_DIR="${HOME}/.airgenome"

say() { printf "\033[1;36m[airgenome]\033[0m %s\n" "$*"; }

if [[ -f "${AGENT_PLIST}" ]]; then
  say "unloading LaunchAgent"
  launchctl unload "${AGENT_PLIST}" 2>/dev/null || true
  rm -f "${AGENT_PLIST}"
fi

if command -v cargo >/dev/null 2>&1; then
  if cargo install --list 2>/dev/null | grep -q '^airgenome '; then
    say "uninstalling airgenome crate"
    cargo uninstall airgenome || true
  fi
fi

say "done. Data directory preserved: ${DATA_DIR}"
echo "  rm -rf ${DATA_DIR}   # to wipe collected vitals"
