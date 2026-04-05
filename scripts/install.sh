#!/usr/bin/env bash
# airgenome — one-shot installer for macOS.
#
#   curl -fsSL https://raw.githubusercontent.com/need-singularity/airgenome/main/scripts/install.sh | bash
#
# Installs the `airgenome` CLI via cargo, creates the data directory,
# and registers a LaunchAgent that runs `airgenome daemon` in the
# background at 60-second intervals.

set -euo pipefail

REPO="https://github.com/need-singularity/airgenome"
AGENT_LABEL="com.airgenome.daemon"
AGENT_PLIST="${HOME}/Library/LaunchAgents/${AGENT_LABEL}.plist"
DATA_DIR="${HOME}/.airgenome"
BIN="${HOME}/.cargo/bin/airgenome"
INTERVAL="${AIRGENOME_INTERVAL:-60}"

say() { printf "\033[1;36m[airgenome]\033[0m %s\n" "$*"; }
err() { printf "\033[1;31m[airgenome]\033[0m %s\n" "$*" >&2; }

if [[ "$(uname -s)" != "Darwin" ]]; then
  err "airgenome is macOS-only (you're on $(uname -s))."
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  err "cargo not found. Install Rust first: https://rustup.rs"
  exit 1
fi

say "installing airgenome from ${REPO} …"
cargo install --git "${REPO}" --force

if [[ ! -x "${BIN}" ]]; then
  err "install completed but ${BIN} not found."
  exit 1
fi

say "data directory: ${DATA_DIR}"
mkdir -p "${DATA_DIR}"

say "writing LaunchAgent: ${AGENT_PLIST}"
mkdir -p "$(dirname "${AGENT_PLIST}")"
cat > "${AGENT_PLIST}" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>${AGENT_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>${BIN}</string>
        <string>daemon</string>
        <string>--interval</string>
        <string>${INTERVAL}</string>
        <string>--output</string>
        <string>${DATA_DIR}/vitals.jsonl</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>${DATA_DIR}/daemon.out.log</string>
    <key>StandardErrorPath</key>
    <string>${DATA_DIR}/daemon.err.log</string>
    <key>WorkingDirectory</key>
    <string>${HOME}</string>
    <key>ProcessType</key>
    <string>Background</string>
    <key>LowPriorityIO</key>
    <true/>
    <key>Nice</key>
    <integer>10</integer>
</dict>
</plist>
PLIST

# reload if already loaded
launchctl unload "${AGENT_PLIST}" 2>/dev/null || true
launchctl load   "${AGENT_PLIST}"

sleep 2
if launchctl list | grep -q "${AGENT_LABEL}"; then
  say "LaunchAgent active (${INTERVAL}s interval)"
else
  err "LaunchAgent did not start — check ${DATA_DIR}/daemon.err.log"
  exit 1
fi

# Register signature + quiet-tune automations via bootstrap.
if command -v "${BIN}" >/dev/null 2>&1; then
  say "registering additional automations (signature + quiet-tune)…"
  "${BIN}" schedule-signature -i 5 >/dev/null 2>&1 || true
  "${BIN}" schedule-quiet -i 1 >/dev/null 2>&1 || true
fi

say "done."
echo
echo "Active automations (all kill-free):"
echo "  com.airgenome.daemon      — 60s vitals log"
echo "  com.airgenome.signature   — 5m per-category accumulation"
echo "  com.airgenome.quiet       — 1h kill-free quiet-tune"
echo
echo "Try:"
echo "  airgenome status                  # hexagon state + vitals"
echo "  airgenome quiet-tune --yes        # run kill-free tuning now"
echo "  airgenome chart --tail 60         # ASCII vitals sparklines"
echo "  airgenome doctor                  # health check"
echo
echo "Tier 2 (optional, needs sudo) — enables purge + Spotlight/TM/DNS levers:"
echo "  curl -fsSL ${REPO}/raw/main/scripts/install-helper.sh | sudo bash -s install"
echo
echo "Uninstall:"
echo "  curl -fsSL ${REPO}/raw/main/scripts/uninstall.sh | bash"
