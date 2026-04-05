#!/usr/bin/env bash
# airgenome-helper — privileged daemon installer (Tier 2).
#
# Installs a LaunchDaemon (runs as root) that listens on a Unix socket.
# The user-level airgenome client connects to it for sysctl operations.
#
# **Safety note (v3.8 skeleton)**: the helper currently REFUSES every
# write and purge operation. It only responds to ping + sysctl_get.
# This installer lays the infrastructure; a future stage will enable
# whitelisted writes once peer-authentication ships.
#
# Usage:
#   sudo bash scripts/install-helper.sh install
#   sudo bash scripts/install-helper.sh uninstall
#   bash scripts/install-helper.sh status     (no sudo required)

set -euo pipefail

LABEL="com.airgenome.helper"
PLIST="/Library/LaunchDaemons/${LABEL}.plist"
BIN_SRC="${HOME}/.cargo/bin/airgenome-helper"
BIN_DST="/usr/local/libexec/airgenome-helper"
SOCK="/var/run/airgenome.sock"
LOG_OUT="/var/log/airgenome-helper.out.log"
LOG_ERR="/var/log/airgenome-helper.err.log"

say() { printf "\033[1;36m[airgenome-helper]\033[0m %s\n" "$*"; }
err() { printf "\033[1;31m[airgenome-helper]\033[0m %s\n" "$*" >&2; }

action="${1:-help}"

case "${action}" in
  install)
    if [[ $EUID -ne 0 ]]; then
      err "install requires sudo."
      exit 1
    fi
    if [[ ! -x "${BIN_SRC}" ]]; then
      # try target/release fallback
      if [[ -x "$(dirname "$0")/../target/release/airgenome-helper" ]]; then
        BIN_SRC="$(cd "$(dirname "$0")/.." && pwd)/target/release/airgenome-helper"
      else
        err "binary not found at ${BIN_SRC}"
        err "run: cargo build --release --bin airgenome-helper"
        exit 1
      fi
    fi
    say "copying ${BIN_SRC} → ${BIN_DST}"
    install -m 755 "${BIN_SRC}" "${BIN_DST}"

    say "writing ${PLIST}"
    cat > "${PLIST}" <<PLISTEOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>${LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>${BIN_DST}</string>
        <string>${SOCK}</string>
    </array>
    <key>RunAtLoad</key><true/>
    <key>KeepAlive</key><true/>
    <key>StandardOutPath</key><string>${LOG_OUT}</string>
    <key>StandardErrorPath</key><string>${LOG_ERR}</string>
    <key>ProcessType</key><string>Background</string>
    <key>LowPriorityIO</key><true/>
    <key>Nice</key><integer>10</integer>
</dict>
</plist>
PLISTEOF
    chmod 644 "${PLIST}"

    say "loading LaunchDaemon"
    launchctl bootstrap system "${PLIST}" 2>/dev/null || launchctl load "${PLIST}"

    sleep 1
    if [[ -S "${SOCK}" ]]; then
      say "helper is running; socket at ${SOCK}"
    else
      err "helper did not create socket ${SOCK}; check ${LOG_ERR}"
      exit 1
    fi
    ;;

  uninstall)
    if [[ $EUID -ne 0 ]]; then
      err "uninstall requires sudo."
      exit 1
    fi
    say "unloading LaunchDaemon"
    launchctl bootout "system/${LABEL}" 2>/dev/null || launchctl unload "${PLIST}" 2>/dev/null || true
    rm -f "${PLIST}" "${BIN_DST}" "${SOCK}"
    say "done."
    ;;

  status)
    if launchctl list 2>/dev/null | grep -q "${LABEL}"; then
      say "helper is loaded"
    else
      say "helper is NOT loaded"
    fi
    if [[ -S "${SOCK}" ]]; then
      say "socket exists: ${SOCK}"
    else
      say "socket missing: ${SOCK}"
    fi
    if [[ -f "${LOG_ERR}" ]]; then tail -5 "${LOG_ERR}" 2>/dev/null || true; fi
    exit 0
    ;;

  help|*)
    echo "usage: $0 {install|uninstall|status}"
    echo "  install    (sudo) copy binary + LaunchDaemon + load"
    echo "  uninstall  (sudo) unload + remove"
    echo "  status     (no sudo) check if loaded / socket present"
    ;;
esac
