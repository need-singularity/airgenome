#!/usr/bin/env bash
# bin/daemons_start.sh — 재부팅 시 1회 실행되어 nohup daemon 들을 기동.
#
# launchd 의 지속형 agent 는 2초 내 unload 되는 현상이 있어 (원인 미파악),
# launchd 는 부팅 시 트리거 역할만 하고, 실제 루프는 nohup 자식 프로세스가 유지.
# 이미 돌고 있으면 중복 기동 방지 — 수동 재실행도 안전.
#
# 관리 대상:
#   - compute-tick: dispatch.selection.compute 로 주기 워크 송신 (ubu2 자원 활용)
#   - remote-load:  ubu/ubu2/hetzner load poll 30s 주기 → remote_load.jsonl
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOG="$HOME/.airgenome"
mkdir -p "$LOG"

start_if_absent() {
    local pattern=$1 cmd=$2 log=$3
    if pgrep -f "$pattern" >/dev/null 2>&1; then
        echo "[skip] $pattern (이미 실행 중)"
        return
    fi
    nohup bash -c "$cmd" >> "$log" 2>&1 &
    disown
    echo "[start] $pattern pid=$!"
}

start_if_absent \
    'while true.*compute_tick' \
    "while true; do bash '$ROOT/bin/compute_tick.sh'; sleep 5; done" \
    "$LOG/compute_tick_loop.log"

start_if_absent \
    'while true.*remote_load' \
    "while true; do bash '$ROOT/bin/remote_load.sh' probe >/dev/null 2>&1; sleep 30; done" \
    "$LOG/remote_load_loop.log"

echo "daemons_start.sh done"
