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

# ═══════════════════════════════════════════════════════════════════════
#  FILTER PIPELINE — 재해석 전용 11-레이어
#  모든 필터는 argv·env·SSOT 만 읽고, 점수만 조정한다 (실행·전송·수정 없음).
#  출력: FILTER_TARGET(local|ubu|htz|vast), FILTER_REASON(stack),
#        FILTER_CONFIDENCE(0..100), FILTER_ENTANGLE_KEY, FILTER_BUDGET_BYPASS
# ═══════════════════════════════════════════════════════════════════════

_AG_SHADOW_FILE="/tmp/ag_route_shadow.jsonl"
_AG_PATHOGEN_FILE="/tmp/ag_gate_pathogens.jsonl"
_AG_HEALTH_FILE="/tmp/ag_host_health.jsonl"
_AG_ENTANGLE_FILE="/tmp/ag_entangle.jsonl"
_AG_LATENCY_LOG="/tmp/ag_gate_latency.log"
_AG_DIFFRACTION_STATE="/tmp/ag_gate_load_state"
_AG_SESSION_NOW="$HOME/.airgenome/session_now.json"
_AG_FILTER_DEBUG="${AG_GATE_DEBUG:-0}"

_ag_reason_push() {
  if [ -z "$FILTER_REASON" ]; then
    FILTER_REASON="$1"
  else
    FILTER_REASON="$FILTER_REASON|$1"
  fi
}

# stable 10-char hash
_ag_hash() {
  printf '%s' "$*" | shasum 2>/dev/null | cut -c1-10
}

# unique / total argv ratio (entropy approximation)
_ag_entropy() {
  _n_total=$#
  [ "$_n_total" -eq 0 ] && { echo 0; return; }
  _n_unique=$(printf '%s\n' "$@" | sort -u | wc -l | tr -d ' ')
  echo $(( _n_unique * 100 / _n_total ))
}

# score mutation helper — only whitelisted targets
_ag_score_add() {
  case "$1" in
    local) SCORE_local=$(( ${SCORE_local:-0} + $2 )) ;;
    ubu)   SCORE_ubu=$((   ${SCORE_ubu:-0}   + $2 )) ;;
    htz)   SCORE_htz=$((   ${SCORE_htz:-0}   + $2 )) ;;
    vast)  SCORE_vast=$((  ${SCORE_vast:-0}  + $2 )) ;;
  esac
}

# ── 1. shadow — past route projection ─────────────────────────────────
ag_filter_shadow() {
  [ -f "$_AG_SHADOW_FILE" ] || return 0
  _line=$(grep "\"k\":\"$1\"" "$_AG_SHADOW_FILE" 2>/dev/null | tail -1)
  [ -z "$_line" ] && return 0
  _t=$(echo "$_line" | sed -e 's/.*"t":"\([^"]*\)".*/\1/')
  _rc=$(echo "$_line" | sed -e 's/.*"rc":\([-0-9]*\).*/\1/')
  if [ "$_rc" = "0" ] && [ -n "$_t" ]; then
    _ag_score_add "$_t" 40
    _ag_reason_push "shadow+:$_t"
  elif [ -n "$_t" ]; then
    _ag_score_add "$_t" -30
    _ag_reason_push "shadow-:$_t"
  fi
}

# ── 2. immune — pathogen short-circuit ────────────────────────────────
ag_filter_immune() {
  [ -f "$_AG_PATHOGEN_FILE" ] || return 0
  if grep -q "\"k\":\"$1\"" "$_AG_PATHOGEN_FILE" 2>/dev/null; then
    FILTER_TARGET="local"
    FILTER_CONFIDENCE=95
    FILTER_SHORT=1
    _ag_reason_push "immune:block"
  fi
}

# ── 3. gravity — data locality pull ───────────────────────────────────
ag_filter_gravity() {
  for a in "$@"; do
    case "$a" in
      /mnt/ramdisk/*)
        _ag_score_add ubu 50
        _ag_reason_push "grav:ramdisk→ubu" ;;
      */target/release/*|*/target/debug/*)
        _ag_score_add local 30
        _ag_reason_push "grav:target→local" ;;
      */forge/by_pid/*)
        _ag_score_add local 20
        _ag_reason_push "grav:forge→local" ;;
      */gpu_*.py)
        _ag_score_add ubu 40
        _ag_reason_push "grav:gpu→ubu" ;;
    esac
  done
}

# ── 4. entangle — parent session inheritance ─────────────────────────
ag_filter_entangle() {
  [ -f "$_AG_ENTANGLE_FILE" ] || return 0
  _pk=""
  if [ -n "$CARGO_PRIMARY_PACKAGE" ]; then
    _pk="cargo-pkg-$CARGO_PRIMARY_PACKAGE"
  elif [ -n "$CARGO_PKG_NAME" ]; then
    _pk="cargo-pkg-$CARGO_PKG_NAME"
  elif [ -n "$AG_ENTANGLE_PARENT" ]; then
    _pk="$AG_ENTANGLE_PARENT"
  elif [ -n "$PPID" ]; then
    _pk="pid-$PPID"
  fi
  [ -z "$_pk" ] && return 0
  _line=$(grep "\"k\":\"$_pk\"" "$_AG_ENTANGLE_FILE" 2>/dev/null | tail -1)
  [ -z "$_line" ] && return 0
  _born=$(echo "$_line" | sed -e 's/.*"born":\([0-9]*\).*/\1/')
  _ttl=$(echo "$_line" | sed -e 's/.*"ttl":\([0-9]*\).*/\1/')
  _now=$(date +%s)
  if [ -n "$_born" ] && [ -n "$_ttl" ] && [ $(( _now - _born )) -lt "$_ttl" ]; then
    _t=$(echo "$_line" | sed -e 's/.*"t":"\([^"]*\)".*/\1/')
    if [ -n "$_t" ]; then
      _ag_score_add "$_t" 60
      FILTER_ENTANGLE_KEY="$_pk"
      _ag_reason_push "entangle:${_pk}>${_t}"
    fi
  fi
}

# ── 5. doppler — latency trend (half-avg diff) ────────────────────────
ag_filter_doppler() {
  [ -f "$_AG_LATENCY_LOG" ] || return 0
  _trend=$(tail -10 "$_AG_LATENCY_LOG" 2>/dev/null | sed -n 's/.*lat_ms=\([0-9][0-9]*\).*/\1/p' | awk '
    { a[NR]=$1; n++ }
    END {
      if (n<4) { print 0; exit }
      h=int(n/2); s1=0; s2=0
      for (i=1;i<=h;i++)      s1+=a[i]
      for (i=h+1;i<=n;i++)    s2+=a[i]
      printf "%d", (s2/(n-h)) - (s1/h)
    }' 2>/dev/null)
  [ -z "$_trend" ] && _trend=0
  if [ "$_trend" -gt 50 ]; then
    _ag_score_add ubu -20
    _ag_score_add htz -20
    _ag_reason_push "doppler:+${_trend}ms"
  elif [ "$_trend" -lt -30 ]; then
    _ag_score_add ubu 10
    _ag_reason_push "doppler:${_trend}ms"
  fi
}

# ── 6. diffraction — load_state hysteresis ───────────────────────────
ag_filter_diffraction() {
  _prev="cool"
  [ -f "$_AG_DIFFRACTION_STATE" ] && _prev=$(cat "$_AG_DIFFRACTION_STATE" 2>/dev/null)
  _ml="${mac_load:-0}"
  _new="$_prev"
  case "$_prev" in
    cool) [ "$_ml" -ge 120 ] && _new="warm"; [ "$_ml" -ge 320 ] && _new="hot" ;;
    warm) [ "$_ml" -lt 80  ] && _new="cool"; [ "$_ml" -ge 320 ] && _new="hot" ;;
    hot)  [ "$_ml" -lt 250 ] && _new="warm" ;;
  esac
  if [ "$_new" != "$_prev" ]; then
    printf '%s' "$_new" > "${_AG_DIFFRACTION_STATE}.tmp" 2>/dev/null && \
      mv "${_AG_DIFFRACTION_STATE}.tmp" "$_AG_DIFFRACTION_STATE" 2>/dev/null
    _ag_reason_push "diffract:${_prev}>${_new}"
  fi
  load_state="$_new"
  case "$load_state" in
    hot)  _ag_score_add ubu 40; _ag_score_add htz 30 ;;
    warm) _ag_score_add ubu 20 ;;
    cool) _ag_score_add local 15 ;;
  esac
}

# ── 7. vacuum — stale observation decay ──────────────────────────────
ag_filter_vacuum() {
  _now=$(date +%s)
  if [ -f "$_AG_SESSION_NOW" ]; then
    _mt=$(stat -f %m "$_AG_SESSION_NOW" 2>/dev/null || echo 0)
    _age=$(( _now - _mt ))
    if [ "$_age" -gt 15 ]; then
      SCORE_ubu=$(( ${SCORE_ubu:-0} / 2 ))
      SCORE_htz=$(( ${SCORE_htz:-0} / 2 ))
      _ag_score_add local 10
      _ag_reason_push "vacuum:session-stale(${_age}s)"
    fi
  else
    _ag_score_add local 5
    _ag_reason_push "vacuum:no-session"
  fi
}

# ── 8. standing_wave — periodicity by hour bucket ────────────────────
ag_filter_standing_wave() {
  _gf="$HOME/Dev/airgenome/forge/per_source_genome.jsonl"
  [ -f "$_gf" ] || return 0
  _hour=$(date +%H)
  _hits=$(grep "\"gate\":\"$1\"" "$_gf" 2>/dev/null | grep -c "\"h\":\"$_hour\"")
  if [ "${_hits:-0}" -gt 3 ]; then
    _ag_score_add ubu 10
    _ag_reason_push "wave:h${_hour}×${_hits}"
  fi
}

# ── 9. interference — burst detection ────────────────────────────────
ag_filter_interference() {
  _n=$(ag_budget_count)
  if [ "${_n:-0}" -ge 5 ]; then
    _ag_score_add ubu 10
    _ag_reason_push "interf:burst($_n)"
  fi
}

# ── 10. tunneling — small-script through budget wall ─────────────────
ag_filter_tunneling() {
  _cap="${AG_BUDGET_CAP:-20}"
  _n=$(ag_budget_count)
  [ "${_n:-0}" -lt "$_cap" ] && return 0
  _all_small=1; _has_files=0
  for a in "$@"; do
    if [ -f "$a" ]; then
      _has_files=1
      _sz=$(stat -f %z "$a" 2>/dev/null || echo 0)
      [ "$_sz" -gt 5120 ] && _all_small=0
    fi
  done
  if [ "$_has_files" = "1" ] && [ "$_all_small" = "1" ]; then
    FILTER_BUDGET_BYPASS=1
    _ag_score_add local 20
    _ag_reason_push "tunnel:small-through-wall"
  fi
}

# ── 11. spin — balance comparable remotes via health ─────────────────
ag_filter_spin() {
  _u=${SCORE_ubu:-0}; _h=${SCORE_htz:-0}
  _d=$(( _u - _h ))
  [ "$_d" -ge 10 ] && return 0
  [ "$_d" -le -10 ] && return 0
  [ -f "$_AG_HEALTH_FILE" ] || return 0
  _ul=$(grep "\"host\":\"ubu\"" "$_AG_HEALTH_FILE" 2>/dev/null | tail -1 | sed -n 's/.*"load":\([0-9][0-9]*\).*/\1/p')
  _hl=$(grep "\"host\":\"htz\"" "$_AG_HEALTH_FILE" 2>/dev/null | tail -1 | sed -n 's/.*"load":\([0-9][0-9]*\).*/\1/p')
  [ -z "$_ul" ] && return 0
  [ -z "$_hl" ] && return 0
  if [ "$_ul" -lt "$_hl" ]; then
    _ag_score_add ubu 15
    _ag_reason_push "spin:ubu-lighter"
  else
    _ag_score_add htz 15
    _ag_reason_push "spin:htz-lighter"
  fi
}

# ── 12. entropy — argv diversity ─────────────────────────────────────
ag_filter_entropy() {
  _ent=$(_ag_entropy "$@")
  if [ "$_ent" -lt 30 ]; then
    _ag_score_add local 10
    _ag_reason_push "entropy:low($_ent%)"
  elif [ "$_ent" -gt 80 ]; then
    _ag_score_add ubu 10
    _ag_reason_push "entropy:high($_ent%)"
  fi
}

# ═══════════════════════════════════════════════════════════════════════
#  SSOT init + pipeline orchestrator
# ═══════════════════════════════════════════════════════════════════════

ag_filter_init_ssot() {
  [ -f "$_AG_SHADOW_FILE" ]   || : > "$_AG_SHADOW_FILE"   2>/dev/null
  [ -f "$_AG_PATHOGEN_FILE" ] || : > "$_AG_PATHOGEN_FILE" 2>/dev/null
  [ -f "$_AG_HEALTH_FILE" ]   || : > "$_AG_HEALTH_FILE"   2>/dev/null
  [ -f "$_AG_ENTANGLE_FILE" ] || : > "$_AG_ENTANGLE_FILE" 2>/dev/null
}

# ag_filter_decide <gate> "$@"
ag_filter_decide() {
  _gate="$1"; shift
  FILTER_TARGET=""
  FILTER_REASON=""
  FILTER_CONFIDENCE=50
  FILTER_ENTANGLE_KEY=""
  FILTER_BUDGET_BYPASS=0
  FILTER_SHORT=0
  SCORE_local=0
  SCORE_ubu=0
  SCORE_htz=0
  SCORE_vast=0

  ag_filter_init_ssot
  FILTER_KEY=$(_ag_hash "$_gate|$PWD|$*")

  ag_filter_immune "$FILTER_KEY"
  [ "$FILTER_SHORT" = "1" ] && return 0

  ag_filter_shadow "$FILTER_KEY"
  ag_filter_gravity "$@"
  ag_filter_entangle
  ag_filter_diffraction
  ag_filter_doppler
  ag_filter_vacuum
  ag_filter_standing_wave "$_gate"
  ag_filter_interference
  ag_filter_tunneling "$@"
  ag_filter_entropy "$@"
  ag_filter_spin

  # argmax — tie-break toward local (safest)
  _best="local"; _bs="$SCORE_local"
  [ "$SCORE_ubu"  -gt "$_bs" ] && { _best="ubu";  _bs="$SCORE_ubu";  }
  [ "$SCORE_htz"  -gt "$_bs" ] && { _best="htz";  _bs="$SCORE_htz";  }
  [ "$SCORE_vast" -gt "$_bs" ] && { _best="vast"; _bs="$SCORE_vast"; }
  FILTER_TARGET="$_best"

  _c="$_bs"
  [ "$_c" -lt 0 ] && _c=0
  [ "$_c" -gt 100 ] && _c=100
  FILTER_CONFIDENCE="$_c"

  if [ "$AG_GATE_DEBUG" = "1" ]; then
    echo "[filter] gate=$_gate target=$FILTER_TARGET conf=$FILTER_CONFIDENCE key=$FILTER_KEY" >&2
    echo "[filter] scores local=$SCORE_local ubu=$SCORE_ubu htz=$SCORE_htz vast=$SCORE_vast" >&2
    echo "[filter] reasons: $FILTER_REASON" >&2
  fi
}

# ag_filter_record <gate> <target> <rc> <lat_ms>
ag_filter_record() {
  _g="$1"; _t="$2"; _rc="$3"; _lat="$4"
  _ts=$(date +%s)
  printf '{"k":"%s","g":"%s","t":"%s","rc":%s,"lat":%s,"ts":%s}\n' \
    "$FILTER_KEY" "$_g" "$_t" "${_rc:-0}" "${_lat:-0}" "$_ts" >> "$_AG_SHADOW_FILE" 2>/dev/null
  # rotate shadow to last 2000 lines
  _lines=$(wc -l < "$_AG_SHADOW_FILE" 2>/dev/null | tr -d ' ')
  if [ "${_lines:-0}" -gt 2500 ]; then
    tail -2000 "$_AG_SHADOW_FILE" > "${_AG_SHADOW_FILE}.tmp" 2>/dev/null && \
      mv "${_AG_SHADOW_FILE}.tmp" "$_AG_SHADOW_FILE" 2>/dev/null
  fi
  # pathogen escalation
  if [ "$_rc" != "0" ]; then
    _fails=$(grep -c "\"k\":\"$FILTER_KEY\"" "$_AG_SHADOW_FILE" 2>/dev/null)
    if [ "${_fails:-0}" -ge 3 ]; then
      if ! grep -q "\"k\":\"$FILTER_KEY\"" "$_AG_PATHOGEN_FILE" 2>/dev/null; then
        printf '{"k":"%s","g":"%s","reason":"repeat-fail","ts":%s}\n' \
          "$FILTER_KEY" "$_g" "$_ts" >> "$_AG_PATHOGEN_FILE" 2>/dev/null
      fi
    fi
  fi
}

# ag_filter_entangle_emit <target> [ttl_sec]
ag_filter_entangle_emit() {
  _t="$1"; _ttl="${2:-300}"
  _born=$(date +%s)
  _key="pid-$$"
  [ -n "$CARGO_PKG_NAME" ] && _key="cargo-pkg-$CARGO_PKG_NAME"
  [ -n "$CARGO_PRIMARY_PACKAGE" ] && _key="cargo-pkg-$CARGO_PRIMARY_PACKAGE"
  printf '{"k":"%s","t":"%s","born":%s,"ttl":%s}\n' \
    "$_key" "$_t" "$_born" "$_ttl" >> "$_AG_ENTANGLE_FILE" 2>/dev/null
  export AG_ENTANGLE_PARENT="$_key"
  FILTER_ENTANGLE_KEY="$_key"
  # prune entangle file
  _lines=$(wc -l < "$_AG_ENTANGLE_FILE" 2>/dev/null | tr -d ' ')
  if [ "${_lines:-0}" -gt 500 ]; then
    tail -300 "$_AG_ENTANGLE_FILE" > "${_AG_ENTANGLE_FILE}.tmp" 2>/dev/null && \
      mv "${_AG_ENTANGLE_FILE}.tmp" "$_AG_ENTANGLE_FILE" 2>/dev/null
  fi
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
