#!/usr/bin/env bash
# bin/executor.sh — dispatch.selection 을 실제 SSH exec 으로 연결하는 MVP.
#
# 책임:
#   1. ~/Dev/nexus/shared/dispatch_state.json 에서 .selection.<kind> 읽기
#   2. shared/config/hosts.json 에서 해당 호스트의 ssh_alias 조회
#   3. local(mac) 또는 ssh <alias> 실행 → stdout/stderr 통과 + exit code 전파
#   4. ~/.airgenome/executor.jsonl 에 1-line 로그
#
# Non-goals (후속): queue / retry / timeout / host affinity / parallel.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DISP="${DISPATCH_STATE:-$HOME/Dev/nexus/shared/dispatch_state.json}"
REG="${HOSTS_REGISTRY:-$ROOT/shared/config/hosts.json}"
LOG="${EXECUTOR_LOG:-$HOME/.airgenome/executor.jsonl}"
mkdir -p "$(dirname "$LOG")"

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
    [ "$fail" = "0" ] && echo "  ✅ executor self_test PASS" || { echo "  ❌ executor self_test FAIL"; exit 1; }
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
    exit 0
fi

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
