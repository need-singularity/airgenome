#!/usr/bin/env bash
# bin/executor.sh — dispatch.selection 을 실제 SSH exec 으로 연결 + slot 하드캡.
#
# 책임:
#   1. ~/Dev/nexus/shared/dispatch_state.json 에서 .selection.<kind> 읽기
#   2. shared/config/hosts.json 에서 해당 호스트의 ssh_alias + slots.<kind> 조회
#   3. slot acquire (flock): active < cap 이면 카운터++, 아니면 MAX_WAIT 까지 재시도
#   4. local(mac) 또는 ssh <alias> 실행 → stdout/stderr 통과 + exit code 전파
#   5. trap EXIT 로 slot release (카운터--)
#   6. ~/.airgenome/executor.jsonl + slots.jsonl 에 1-line 로그
#
# Why slot cap: 같은 host 에 heavy 2개 동시 dispatch 되면 12코어를 나눠먹어 1h → 1.5h.
# slots.heavy=1 로 하드캡 → 직렬화. cap 초과 시 exit 75 (EX_TEMPFAIL) → caller 재dispatch.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DISP="${DISPATCH_STATE:-$HOME/Dev/nexus/shared/dispatch_state.json}"
REG="${HOSTS_REGISTRY:-$ROOT/shared/config/hosts.json}"
LOG="${EXECUTOR_LOG:-$HOME/.airgenome/executor.jsonl}"
SLOTS_DIR="${SLOTS_DIR:-$HOME/.airgenome/host_slots}"
SLOTS_LOG="${SLOTS_LOG:-$HOME/.airgenome/slots.jsonl}"
MAX_WAIT="${EXECUTOR_MAX_WAIT:-120}"
WAIT_STEP="${EXECUTOR_WAIT_STEP:-5}"
mkdir -p "$(dirname "$LOG")" "$SLOTS_DIR"

die() { echo "executor: $*" >&2; exit 2; }
usage() {
    sed -n '1,20p' "$0" | sed -n '/^# /p' >&2
    exit 2
}

# kind → selected host key (e.g., "ubu2") / "none"
resolve_host() {
    local kind=$1
    [ -r "$DISP" ] || die "dispatch_state 없음: $DISP"
    jq -r --arg k "$kind" '.selection[$k] // "none"' "$DISP"
}

# host key → {kind, ssh_alias}
resolve_alias() {
    local host=$1
    [ -r "$REG" ] || die "registry 없음: $REG"
    jq -r --arg h "$host" '.hosts[$h] // empty | "\(.kind)\t\(.ssh_alias // "")"' "$REG"
}

log_jsonl() {
    # fields: ts, kind, host, alias, exit, ms, cmd
    local ts kind host alias exit_code ms cmd
    ts=$1; kind=$2; host=$3; alias=$4; exit_code=$5; ms=$6; cmd=$7
    local cmd_esc
    cmd_esc=$(printf '%s' "$cmd" | jq -Rs .)
    printf '{"ts":"%s","kind":"%s","host":"%s","alias":"%s","exit":%s,"ms":%s,"cmd":%s}\n' \
        "$ts" "$kind" "$host" "$alias" "$exit_code" "$ms" "$cmd_esc" >> "$LOG"
}

# ── slot management (A+B: active counter + mkdir-lock 하드 캡) ──
# macOS 는 flock 비표준 → mkdir (POSIX atomic) 으로 mutex. Linux 동일 동작.
#
# slot cap 조회. 필드 누락 → -1 (무제한). 0 → 금지.
slot_cap() {
    local host=$1 kind=$2
    jq -r --arg h "$host" --arg k "$kind" \
        '.hosts[$h].slots[$k] // -1' "$REG"
}

# empty → 0 정규화 (slots_log 입력 보호)
_n() { local v=${1:-0}; [[ "$v" =~ ^-?[0-9]+$ ]] && echo "$v" || echo 0; }

slots_log() {
    local ts ev host kind active cap wait_ms
    ts=$1; ev=$2; host=$3; kind=$4
    active=$(_n "${5:-}")
    cap=$(_n "${6:--1}")
    wait_ms=$(_n "${7:-0}")
    printf '{"ts":"%s","ev":"%s","host":"%s","kind":"%s","active":%s,"cap":%s,"wait_ms":%s}\n' \
        "$ts" "$ev" "$host" "$kind" "$active" "$cap" "$wait_ms" >> "$SLOTS_LOG"
}

# mkdir 기반 mutex. 성공=0, 실패=1. LOCK_SPIN_MAX 회 시도 (~1s).
_mutex_acquire() {
    local lockdir=$1
    local i
    for i in $(seq 1 20); do
        if mkdir "$lockdir" 2>/dev/null; then
            return 0
        fi
        sleep 0.05
    done
    return 1
}
_mutex_release() {
    rmdir "$1" 2>/dev/null || true
}

# 카운터 읽기 (파일 없음/빈값/비숫자 → 0)
_read_counter() {
    local counter=$1
    local current=0
    [ -f "$counter" ] && current=$(cat "$counter" 2>/dev/null || echo 0)
    current=${current:-0}
    [[ "$current" =~ ^[0-9]+$ ]] || current=0
    echo "$current"
}

# acquire_slot <host> <kind>
#   반환: 0=획득, 1=cap=0(금지), 2=timeout
#   set -e 피해 inline 테스트 권장: `acquire_slot x y || rc_acq=$?`
acquire_slot() {
    local host=$1 kind=$2
    local cap; cap=$(slot_cap "$host" "$kind")
    if [ "$cap" = "-1" ] || [ -z "$cap" ]; then
        return 0  # 무제한
    fi
    if [ "$cap" = "0" ]; then
        slots_log "$(date -u +%FT%TZ)" "forbidden" "$host" "$kind" 0 0 0
        return 1
    fi
    local lockdir="$SLOTS_DIR/$host.$kind.lockdir"
    local counter="$SLOTS_DIR/$host.$kind.active"
    touch "$counter"
    local t_start; t_start=$(date +%s)
    local waited=0
    local last_log=-1
    while : ; do
        if _mutex_acquire "$lockdir"; then
            local current; current=$(_read_counter "$counter")
            if [ "$current" -lt "$cap" ]; then
                local new=$((current + 1))
                echo "$new" > "$counter"
                _mutex_release "$lockdir"
                slots_log "$(date -u +%FT%TZ)" "acquire" "$host" "$kind" "$new" "$cap" "$((waited * 1000))"
                _SLOT_HOST=$host
                _SLOT_KIND=$kind
                return 0
            fi
            _mutex_release "$lockdir"
            local now; now=$(date +%s)
            waited=$((now - t_start))
            if [ "$waited" -ge "$MAX_WAIT" ]; then
                slots_log "$(date -u +%FT%TZ)" "timeout" "$host" "$kind" "$current" "$cap" "$((waited * 1000))"
                return 2
            fi
            # WAIT_STEP 마다 한 번만 deferred 로그 (중복 방지)
            local bucket=$((waited / WAIT_STEP))
            if [ "$bucket" != "$last_log" ]; then
                slots_log "$(date -u +%FT%TZ)" "deferred" "$host" "$kind" "$current" "$cap" "$((waited * 1000))"
                last_log=$bucket
            fi
        else
            # mutex 획득 실패 — 다른 프로세스가 잡고 있음. 그냥 재시도.
            :
        fi
        sleep "$WAIT_STEP"
    done
}

# release_slot — trap 에서 호출. 전역 _SLOT_HOST/_SLOT_KIND 기반.
release_slot() {
    [ -z "${_SLOT_HOST:-}" ] && return 0
    [ -z "${_SLOT_KIND:-}" ] && return 0
    local lockdir="$SLOTS_DIR/$_SLOT_HOST.$_SLOT_KIND.lockdir"
    local counter="$SLOTS_DIR/$_SLOT_HOST.$_SLOT_KIND.active"
    if _mutex_acquire "$lockdir"; then
        local current; current=$(_read_counter "$counter")
        if [ "$current" -gt 0 ]; then
            echo $((current - 1)) > "$counter"
        else
            echo 0 > "$counter"
        fi
        local new; new=$(_read_counter "$counter")
        _mutex_release "$lockdir"
        local cap; cap=$(slot_cap "$_SLOT_HOST" "$_SLOT_KIND" 2>/dev/null || echo -1)
        slots_log "$(date -u +%FT%TZ)" "release" "$_SLOT_HOST" "$_SLOT_KIND" "$new" "$cap" 0
    fi
    _SLOT_HOST=""
    _SLOT_KIND=""
}

# ── self-test ────────────────────────────────────────────────────
self_test() {
    echo "executor self-test"
    local fail=0
    for kind in compute gpu heavy mac_only; do
        local host alias info
        host=$(resolve_host "$kind") || { echo "  FAIL resolve_host $kind"; fail=1; continue; }
        if [ "$host" = "none" ]; then
            echo "  SKIP $kind: selection=none"
            continue
        fi
        info=$(resolve_alias "$host")
        if [ -z "$info" ]; then
            echo "  FAIL $kind: host '$host' registry 에 없음"; fail=1; continue
        fi
        local kind_h alias_h
        kind_h=$(printf '%s' "$info" | cut -f1)
        alias_h=$(printf '%s' "$info" | cut -f2)
        echo "  OK $kind → $host ($kind_h, alias=${alias_h:-self})"
    done
    # slot schema 검증: 모든 enabled host 에 slots 필드 존재
    local schema_fail=0
    while read -r host; do
        local slots; slots=$(jq -r --arg h "$host" '.hosts[$h].slots // "MISSING"' "$REG")
        if [ "$slots" = "MISSING" ]; then
            echo "  FAIL slots schema: $host 에 slots 필드 없음"; schema_fail=1
        fi
    done < <(jq -r '.hosts | to_entries[] | select(.value.enabled) | .key' "$REG")
    [ "$schema_fail" = "0" ] && echo "  OK slots schema: 모든 host 에 slots 필드 존재"
    # slot cap 조회 라운드트립
    local ubu_heavy; ubu_heavy=$(slot_cap "ubu" "heavy")
    [ "$ubu_heavy" = "1" ] || { echo "  FAIL slot_cap ubu.heavy=$ubu_heavy (expect 1)"; fail=1; }
    local mac_heavy; mac_heavy=$(slot_cap "mac" "heavy")
    [ "$mac_heavy" = "0" ] || { echo "  FAIL slot_cap mac.heavy=$mac_heavy (expect 0)"; fail=1; }
    local ghost_cap; ghost_cap=$(slot_cap "nonexistent" "heavy")
    [ "$ghost_cap" = "-1" ] || { echo "  FAIL slot_cap unknown-host=$ghost_cap (expect -1)"; fail=1; }
    [ "$schema_fail" = "0" ] && [ "$fail" = "0" ] && echo "  ✅ executor self_test PASS" || { echo "  ❌ executor self_test FAIL"; exit 1; }
}

# ── main dispatch ────────────────────────────────────────────────
dry=false
if [ "${1:-}" = "--dry" ]; then dry=true; shift; fi
case "${1:-}" in
    ""|-h|--help) usage ;;
    --self-test) self_test; exit 0 ;;
esac

kind=$1; shift || true
[ $# -gt 0 ] || die "명령 누락. usage: executor.sh <kind> <cmd>"
cmd="$*"

host=$(resolve_host "$kind")
[ "$host" = "none" ] && die "dispatch.selection.$kind = none — 가용 호스트 없음"
info=$(resolve_alias "$host")
[ -n "$info" ] || die "host '$host' 가 hosts.json 에 없음"
host_kind=$(printf '%s' "$info" | cut -f1)
alias=$(printf '%s' "$info" | cut -f2)

if [ "$dry" = "true" ]; then
    echo "DRY kind=$kind host=$host host_kind=$host_kind alias=${alias:-(self)}"
    echo "DRY cmd: $cmd"
    local_cap=$(slot_cap "$host" "$kind")
    echo "DRY slot_cap: $host.$kind=$local_cap"
    exit 0
fi

# slot acquire — cap 초과 시 MAX_WAIT 후 exit 75 (EX_TEMPFAIL)
# mac_only 는 slot 대상 아님 (compute/heavy/gpu 만)
_SLOT_HOST=""
_SLOT_KIND=""
trap 'release_slot' EXIT
case "$kind" in
    compute|heavy|gpu)
        # set -e 회피 — acquire_slot 의 non-zero return 이 스크립트 abort 시키지 않도록 inline.
        rc_acq=0
        acquire_slot "$host" "$kind" || rc_acq=$?
        case $rc_acq in
            0) : ;;  # 획득 성공 (또는 무제한)
            1) die "slot 금지: $host.$kind=0 (registry 에서 차단됨)" ;;
            2) echo "executor: slot timeout — $host.$kind cap 초과 ${MAX_WAIT}s 대기 실패" >&2; exit 75 ;;
        esac
        ;;
esac

ts=$(date -u +%FT%TZ)
t0=$(date +%s)
if [ "$host_kind" = "self" ]; then
    bash -c "$cmd"
    rc=$?
else
    [ -n "$alias" ] || die "host '$host' ssh_alias 비어있음"
    ssh -o ConnectTimeout=5 "$alias" "$cmd"
    rc=$?
fi
t1=$(date +%s)
ms=$(( (t1 - t0) * 1000 ))
log_jsonl "$ts" "$kind" "$host" "${alias:-self}" "$rc" "$ms" "$cmd"
exit $rc
