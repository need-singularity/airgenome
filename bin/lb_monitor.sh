#!/usr/bin/env bash
# bin/lb_monitor.sh — lb 분산 상태 관측 (one-shot + watch).
#
# 집계: ~/.airgenome/lb.jsonl 의 최근 N 엔트리 → host 별 (count, share, avg_ms, fail%)
# 현재: ~/Dev/nexus/shared/lb_state.json scores + chosen + host snapshot
#
# 편향 지표:
#   max_share : 최고 점유 호스트 비율 (0.00 ~ 1.00).
#   zero_host : 0 회 배정된 enabled 호스트. 수가 많을수록 편중.
#
# Commands:
#   lb_monitor.sh [N]                            one-shot (기본 N=20)
#   lb_monitor.sh watch [N] [interval] [stop]    interval 초마다 반복.
#       stop = converge (기본) → max_share==1.00 이 연속 3회 감지 시 종료.
#       stop = never                           → 계속 (Ctrl-C).

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOG="${LB_LOG:-$HOME/.airgenome/lb.jsonl}"
STATE="${LB_STATE:-$HOME/Dev/nexus/shared/lb_state.json}"
REG="${HOSTS_REGISTRY:-$ROOT/shared/config/hosts.json}"

candidates() {
    jq -r '.hosts | to_entries[] | select(.value.enabled == true and .value.kind != "self") | .key' "$REG"
}

# snapshot: 현재 분산 스냅샷 print + max_share 값을 stderr-safe 파일로 전파.
# stdout: 표 + 요약.  exit via echo into /tmp tmp file? 대신 FD 3 사용.
# 단순화: 함수 맨 마지막 줄에 "MAX_SHARE=<val>" 붙여 caller 가 parse.
snapshot() {
    local n=${1:-20}
    [ -r "$LOG" ] || { echo "no log: $LOG" >&2; echo "MAX_SHARE=0"; return; }

    local total window wcnt
    total=$(wc -l < "$LOG" | tr -d ' ')
    window=$(tail -n "$n" "$LOG")
    wcnt=$(printf '%s\n' "$window" | grep -c '^{' || true)

    echo "== lb_monitor $(date -u +%FT%TZ) (window=$wcnt / total=$total) =="
    printf '%-6s %6s %8s %10s %8s\n' HOST COUNT SHARE AVG_MS FAIL%

    local max_share_val=0 zero_hosts=""
    while IFS= read -r h; do
        [ -z "$h" ] && continue
        local cnt share avg_ms fails fail_pct
        cnt=$(printf '%s\n' "$window" | jq -r --arg h "$h" 'select(.host==$h) | .host' 2>/dev/null | wc -l | tr -d ' ')
        if [ "$cnt" = "0" ]; then
            zero_hosts="$zero_hosts $h"
            printf '%-6s %6d %8s %10s %8s\n' "$h" 0 "0.00" "-" "-"
            continue
        fi
        share=$(awk -v c="$cnt" -v w="$wcnt" 'BEGIN{ if (w>0) printf "%.2f", c/w; else print "0.00" }')
        avg_ms=$(printf '%s\n' "$window" | jq -r --arg h "$h" 'select(.host==$h) | .ms' 2>/dev/null | awk '{s+=$1; n++} END{ if (n>0) printf "%.0f", s/n; else print "-" }')
        fails=$(printf '%s\n' "$window" | jq -r --arg h "$h" 'select(.host==$h and .exit != 0) | .host' 2>/dev/null | wc -l | tr -d ' ')
        fail_pct=$(awk -v f="$fails" -v c="$cnt" 'BEGIN{ if (c>0) printf "%.0f", 100*f/c; else print "0" }')
        printf '%-6s %6d %8s %10s %7s%%\n' "$h" "$cnt" "$share" "$avg_ms" "$fail_pct"
        if awk -v a="$share" -v b="$max_share_val" 'BEGIN{ exit !(a > b) }'; then
            max_share_val=$share
        fi
    done < <(candidates)

    echo
    printf 'max_share=%s' "$max_share_val"
    [ -n "$zero_hosts" ] && printf '  zero_hosts=%s' "$(echo "$zero_hosts" | xargs)"
    echo
    if [ -r "$STATE" ]; then
        jq -r '
            "state: ts=\(.ts) kind=\(.kind) chosen=\(.chosen) scores=\(.scores | to_entries | map("\(.key)=\(.value)") | join(","))"
        ' "$STATE"
    fi
    echo
    # 마지막 줄: max_share 값 (caller parse 용 sentinel)
    echo "MAX_SHARE=$max_share_val"
}

cmd_watch() {
    local n=${1:-20} interval=${2:-15} stop=${3:-converge}
    local streak=0
    local need=3   # converge streak threshold
    while true; do
        local out max
        out=$(snapshot "$n")
        # strip sentinel for display, capture for logic
        max=$(printf '%s\n' "$out" | awk -F= '/^MAX_SHARE=/{print $2}')
        printf '%s\n' "$out" | grep -v '^MAX_SHARE='
        if [ "$stop" = "converge" ]; then
            if awk -v v="$max" 'BEGIN{ exit !(v+0 >= 1.0) }'; then
                streak=$((streak + 1))
                echo "(converge streak=$streak/$need)"
                if [ "$streak" -ge "$need" ]; then
                    echo "→ converged: max_share=1.00 × $streak. monitoring stopped."
                    break
                fi
            else
                streak=0
            fi
        fi
        sleep "$interval"
    done
}

case "${1:-}" in
    watch)
        shift
        cmd_watch "${1:-20}" "${2:-15}" "${3:-converge}"
        ;;
    ""|[0-9]*)
        out=$(snapshot "${1:-20}")
        printf '%s\n' "$out" | grep -v '^MAX_SHARE='
        ;;
    -h|--help) sed -n '1,20p' "$0" | sed -n '/^# /p' >&2; exit 2 ;;
    *) echo "unknown: $1" >&2; exit 2 ;;
esac
