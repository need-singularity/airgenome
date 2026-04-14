#!/usr/bin/env bash
# bin/remote_load.sh — Phase R2: remote host load poller
#
# 목적: ubu + hetzner 의 load/mem/proc counts 를 주기적으로 수집해
#       ~/.airgenome/remote_load.jsonl 에 1-host-당-1-line JSONL 로 기록.
# 용도: 이후 dispatcher/circuit-breaker 의 판단 근거, 간섭 분석.
#
# Commands:
#   probe        one-shot probe. 각 host 에 ssh 1회 → JSONL 1줄씩 append.
#   show [N]     tail 마지막 N (기본 10) pretty print.
#   --self-test  probe 실행 + 결과 검증 (양쪽 host 응답 + 필수 필드 확인).
set -eu

LOG="${REMOTE_LOAD_LOG:-$HOME/.airgenome/remote_load.jsonl}"
mkdir -p "$(dirname "$LOG")"

HOSTS=("ubu" "hetzner")

# 원격에서 실행되는 한 줄 JSON 생성기. single-quoted 로 로컬 확장 방지.
# NOTE: `pgrep -c` 는 no-match 시 stdout 에 "0" 출력 + exit 1.
#       따라서 `|| echo 0` 붙이면 "0\n0" 이 되어 값에 newline 박힘. $() 로만 잡고 default fallback 에 맡길 것.
REMOTE_CMD='
    read load1 load5 load15 _rest < /proc/loadavg 2>/dev/null || { load1=0; load5=0; load15=0; }
    memfree=$(awk "/^MemAvailable:/{print \$2; exit}" /proc/meminfo 2>/dev/null)
    memtotal=$(awk "/^MemTotal:/{print \$2; exit}" /proc/meminfo 2>/dev/null)
    nproc_c=$(nproc 2>/dev/null)
    hexa_run=$(pgrep -cf "hexa run" 2>/dev/null)
    hexa_stage0=$(pgrep -xc hexa_stage0 2>/dev/null)
    openssl_c=$(pgrep -xc openssl 2>/dev/null)
    claude_c=$(pgrep -cf "claude" 2>/dev/null)
    blowup=$(pgrep -cf "blowup.hexa" 2>/dev/null)
    : ${load1:=0} ${load5:=0} ${load15:=0}
    : ${memfree:=0} ${memtotal:=1}
    : ${nproc_c:=0} ${hexa_run:=0} ${hexa_stage0:=0}
    : ${openssl_c:=0} ${claude_c:=0} ${blowup:=0}
    printf "{\"load1\":%s,\"load5\":%s,\"load15\":%s,\"memfree_kb\":%s,\"memtotal_kb\":%s,\"nproc\":%s,\"hexa_run\":%s,\"hexa_stage0\":%s,\"openssl\":%s,\"blowup\":%s,\"claude\":%s}\n" \
        "$load1" "$load5" "$load15" "$memfree" "$memtotal" "$nproc_c" "$hexa_run" "$hexa_stage0" "$openssl_c" "$blowup" "$claude_c"
'

probe_host() {
    local host=$1
    local ts
    ts=$(date -u +%FT%TZ)
    local json
    if json=$(ssh -o ConnectTimeout=5 -o BatchMode=yes "$host" "$REMOTE_CMD" 2>/dev/null); then
        # merge: prefix ts/host/ok, append remote json fields (strip opening brace)
        printf '{"ts":"%s","host":"%s","ok":true,%s\n' "$ts" "$host" "${json#\{}" >> "$LOG"
    else
        printf '{"ts":"%s","host":"%s","ok":false,"err":"ssh_fail_or_timeout"}\n' "$ts" "$host" >> "$LOG"
        return 1
    fi
}

cmd_probe() {
    local rc=0
    for h in "${HOSTS[@]}"; do
        probe_host "$h" &
    done
    wait
    return $rc
}

cmd_show() {
    local n=${1:-10}
    [ -s "$LOG" ] || { echo "(log empty: $LOG)"; return 0; }
    if command -v jq >/dev/null 2>&1; then
        tail -n "$n" "$LOG" | jq -cr '. as $r | [.ts, .host, (.ok|tostring), (.load1//"-"), (.nproc//"-"), (.hexa_run//"-"), (.hexa_stage0//"-"), (.openssl//"-"), (.blowup//"-")] | @tsv' 2>/dev/null | column -t -s $'\t' || tail -n "$n" "$LOG"
    else
        tail -n "$n" "$LOG"
    fi
}

self_test() {
    echo "remote_load.sh self-test"
    # 기존 잘못된 로그 초기화 (버그 수정 전 남은 다중라인 JSON 제거)
    touch "$LOG"
    local before after
    before=$(wc -l < "$LOG")
    cmd_probe
    after=$(wc -l < "$LOG" 2>/dev/null || echo 0)
    local delta=$((after - before))
    if [ "$delta" -ne 2 ]; then
        echo "  FAIL: expected 2 new lines (ubu+hetzner), got $delta"; exit 1
    fi
    # 마지막 2줄 유효성 검사
    local any_ok=0
    tail -n 2 "$LOG" | while IFS= read -r l; do
        case "$l" in
            *'"ok":true'*'"load1"'*) echo "  PASS ok: $(echo "$l" | cut -c1-100)..." ;;
            *'"ok":false'*) echo "  WARN unreachable: $l" ;;
            *) echo "  FAIL malformed: $l"; exit 1 ;;
        esac
    done
    # 최소 1 host 는 reachable 해야 통과
    if ! tail -n 2 "$LOG" | grep -q '"ok":true'; then
        echo "  FAIL: 모든 host unreachable"; exit 1
    fi
    echo "  ✅ remote_load self_test PASS"
}

case "${1:-}" in
    probe) cmd_probe ;;
    show) cmd_show "${2:-10}" ;;
    --self-test) self_test ;;
    "") echo "usage: $0 {probe|show [N]|--self-test}" >&2; exit 2 ;;
    *) echo "unknown command: $1" >&2; exit 2 ;;
esac
