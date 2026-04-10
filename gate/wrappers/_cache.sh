#!/bin/bash
# gate/wrappers/_cache.sh — shared caching for nc reachability + budget counter
# Source this from wrapper scripts. Requires UBU_HOST, UBU_PORT, TS_HOST to be set.

_AG_REACH_CACHE="/tmp/ag_gate_reachable.state"
_AG_REACH_TTL=60
_AG_BUDGET_FILE="/tmp/ag_gate_budget.count"
_AG_BUDGET_LOCK="/tmp/ag_gate_budget.lock"

# ── NC reachability cache (60s TTL) ──
# Sets GH to reachable host or "" if none reachable.
# Falls back to live nc check if cache is stale/missing.
ag_resolve_host() {
  GH=""
  if [ -f "$_AG_REACH_CACHE" ]; then
    _age=$(( $(date +%s) - $(stat -f %m "$_AG_REACH_CACHE" 2>/dev/null || echo 0) ))
    if [ "$_age" -lt "$_AG_REACH_TTL" ]; then
      _cached=$(cat "$_AG_REACH_CACHE" 2>/dev/null)
      _ch=$(echo "$_cached" | cut -d'|' -f1)
      _cp=$(echo "$_cached" | cut -d'|' -f2)
      # Only use cache if port matches current config
      if [ "$_cp" = "$UBU_PORT" ] && [ -n "$_ch" ] && [ "$_ch" != "NONE" ]; then
        GH="$_ch"
        return 0
      elif [ "$_ch" = "NONE" ] && [ "$_cp" = "$UBU_PORT" ]; then
        GH=""
        return 0
      fi
    fi
  fi
  # Cache miss or stale — do live nc check (ubu → tailscale → hetzner → vast)
  if nc -z -w 1 "$UBU_HOST" "$UBU_PORT" 2>/dev/null; then
    GH="$UBU_HOST"
  elif [ -n "$TS_HOST" ] && nc -z -w 2 "$TS_HOST" "$UBU_PORT" 2>/dev/null; then
    GH="$TS_HOST"
  elif nc -z -w 2 157.180.8.154 "$UBU_PORT" 2>/dev/null; then
    GH="157.180.8.154"; GH_NAME="hetzner"
  elif nc -z -w 3 ssh9.vast.ai 19200 2>/dev/null; then
    GH="ssh9.vast.ai"; GH_PORT=19200; GH_NAME="vast"
  fi
  # Write cache atomically
  _val="${GH:-NONE}|${UBU_PORT}"
  printf '%s' "$_val" > "${_AG_REACH_CACHE}.tmp" && mv "${_AG_REACH_CACHE}.tmp" "$_AG_REACH_CACHE"
}

# ── Budget counter (replaces per-invocation pgrep) ──
# Uses a simple counter file instead of pgrep -f on every call.
# Wrappers call ag_budget_enter on start, ag_budget_leave on exit (via trap).
ag_budget_count() {
  if [ -f "$_AG_BUDGET_FILE" ]; then
    cat "$_AG_BUDGET_FILE" 2>/dev/null | wc -l | tr -d ' '
  else
    echo 0
  fi
}

ag_budget_enter() {
  _my_pid=$$
  # Append our PID (one per line) — atomic enough for this use case
  echo "$_my_pid" >> "$_AG_BUDGET_FILE"
  # Set trap to clean up on exit
  trap 'ag_budget_leave' EXIT INT TERM
}

ag_budget_leave() {
  _my_pid=$$
  # Remove our PID line atomically
  if [ -f "$_AG_BUDGET_FILE" ]; then
    grep -v "^${_my_pid}$" "$_AG_BUDGET_FILE" > "${_AG_BUDGET_FILE}.tmp" 2>/dev/null
    mv "${_AG_BUDGET_FILE}.tmp" "$_AG_BUDGET_FILE" 2>/dev/null
  fi
}

# Prune stale PIDs (dead processes) — call periodically to avoid drift
ag_budget_prune() {
  [ -f "$_AG_BUDGET_FILE" ] || return
  _tmp="${_AG_BUDGET_FILE}.prune"
  while IFS= read -r _pid; do
    [ -n "$_pid" ] && kill -0 "$_pid" 2>/dev/null && echo "$_pid"
  done < "$_AG_BUDGET_FILE" > "$_tmp"
  mv "$_tmp" "$_AG_BUDGET_FILE" 2>/dev/null
}

# ── AI-native gate banner ──
# Emit structured prefix so AI agents understand dispatch context.
# Usage: ag_gate_msg "remote" "cargo test" "192.168.50.119:9900" "/tmp/airgenome"
#        ag_gate_msg "local"  "cargo test"
#        ag_gate_msg "unreachable" "cargo test"
ag_gate_msg() {
  _mode="$1"; _cmd="$2"; _host="$3"; _rdir="$4"
  case "$_mode" in
    remote)
      echo "[GATE] dispatch=remote host=$_host remote_dir=$_rdir cmd=\"$_cmd\"" >&2
      echo "[GATE] ⚠ paths in output below are REMOTE — not local filesystem." >&2
      ;;
    local)
      echo "[GATE] dispatch=local cmd=\"$_cmd\"" >&2
      ;;
    unreachable)
      echo "[GATE] dispatch=local reason=remote_unreachable cmd=\"$_cmd\"" >&2
      ;;
  esac
}

ag_budget_check() {
  _cap="${AG_BUDGET_CAP:-20}"
  # Prune stale PIDs every ~10th call (random chance)
  _r=$(( $$ % 10 ))
  [ "$_r" -eq 0 ] && ag_budget_prune
  _n=$(ag_budget_count)
  [ "${_n:-0}" -ge "$_cap" ]
}

# ── Config caching (skip awk re-parsing if file unchanged) ──
_AG_CFG_CACHE="/tmp/ag_gate_config.cache"

# ag_load_config CFG_PATH
# Sets UBU_HOST, UBU_PORT, UBU_SSH, UBU_DIR, TS_HOST from cache if config mtime unchanged.
ag_load_config() {
  _cfg="$1"
  [ -f "$_cfg" ] || return
  _cfg_mtime=$(stat -f %m "$_cfg" 2>/dev/null || echo 0)
  # Check if cache exists and mtime matches
  if [ -f "$_AG_CFG_CACHE" ]; then
    _cached_mtime=$(head -1 "$_AG_CFG_CACHE" 2>/dev/null)
    if [ "$_cached_mtime" = "$_cfg_mtime" ]; then
      # Source cached values (skip mtime line via tail)
      eval "$(tail -n +2 "$_AG_CFG_CACHE")"
      return
    fi
  fi
  # Parse fresh and write cache
  _h=$(awk -F'"' '/"remote_host"/{print $8}' "$_cfg")
  _p=$(awk -F'"' '/"remote_port"/{print $8}' "$_cfg")
  _s=$(awk -F'"' '/"ssh_alias"/{print $8}' "$_cfg")
  _d=$(awk -F'"' '/"remote_dir"/{print $8}' "$_cfg")
  _t=$(awk -F'"' '/"tailscale_host"/{print $8}' "$_cfg")
  [ -n "$_h" ] && UBU_HOST="$_h"
  [ -n "$_p" ] && UBU_PORT="$_p"
  [ -n "$_s" ] && UBU_SSH="$_s"
  [ -n "$_d" ] && UBU_DIR="$_d"
  [ -n "$_t" ] && TS_HOST="$_t"
  # Write cache atomically
  {
    echo "$_cfg_mtime"
    echo "UBU_HOST='$UBU_HOST'"
    echo "UBU_PORT='$UBU_PORT'"
    echo "UBU_SSH='$UBU_SSH'"
    echo "UBU_DIR='$UBU_DIR'"
    echo "TS_HOST='$TS_HOST'"
  } > "${_AG_CFG_CACHE}.tmp" && mv "${_AG_CFG_CACHE}.tmp" "$_AG_CFG_CACHE"
}
