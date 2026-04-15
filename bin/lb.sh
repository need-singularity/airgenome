#!/usr/bin/env bash
# bin/lb.sh — mac 클라이언트용 로드밸런서 (3-layer orchestrator)
#
# 아키텍처 (3-layer):  mac (this) → lb.sh → { ubu, ubu2, htz }
# 데이터 소스:         ~/.airgenome/remote_load.jsonl  (remote_load.sh 30s 폴 JSONL)
# 상태 출력:           ~/Dev/nexus/shared/lb_state.json (atomic write per pick)
# 실행 로그:           ~/.airgenome/lb.jsonl  (1 line per run)
# 호스트 레지스트리:    shared/config/hosts.json  (kind != self, enabled == true)
#
# 점수 (centi-thread 정수 — load 소수까지 반영):
#   free_ci = (nproc * 100) − (load1 * 100)  , clamp ≥ 0
#   compute / heavy →  free_ci              # 순수 유휴 스레드
#   gpu            →  has_gpu 호스트만, free_ci (무GPU 호스트는 0)
#
# Fresh gate:   한 호스트의 마지막 remote_load 엔트리 age > 120s → 후보 제외
# Tie-breaker:  등가 점수 시 순서 고정 (ubu → ubu2 → htz). centi 단위에선 실전 tie 드묾.
# 하드코딩 아님: 점수는 실시간 load1/nproc 에서 직접 산출. kind/host 가중치 없음.
#
# Commands:
#   lb.sh pick <kind>               stdout: ubu|ubu2|htz|none
#   lb.sh status                    모든 호스트 snapshot + 3 kind 선택 요약
#   lb.sh run <kind> <cmd...>       pick → ssh 실행 + jsonl 로그 + exit code 전파
#   lb.sh --self-test               단위 테스트 (점수/선택 로직)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REG="${HOSTS_REGISTRY:-$ROOT/shared/config/hosts.json}"
FEED="${LB_FEED:-$HOME/.airgenome/remote_load.jsonl}"
STATE="${LB_STATE:-$HOME/Dev/nexus/shared/lb_state.json}"
LOG="${LB_LOG:-$HOME/.airgenome/lb.jsonl}"
FRESH_S="${LB_FRESH_S:-120}"

mkdir -p "$(dirname "$LOG")"

die() { echo "lb: $*" >&2; exit 2; }

# 호스트 → ssh alias (hosts.json SSOT).
host_to_alias() {
    [ -r "$REG" ] || die "registry 없음: $REG"
    jq -r --arg h "$1" '.hosts[$h].ssh_alias // empty' "$REG"
}

# hosts.json 에서 후보 host-key 목록 (kind != self, enabled == true)
candidate_keys() {
    [ -r "$REG" ] || die "registry 없음: $REG"
    jq -r '.hosts | to_entries[] | select(.value.enabled == true and .value.kind != "self") | .key' "$REG"
}

# remote_load.jsonl 에서 해당 alias 의 마지막 엔트리를 "ts|ok|load1|nproc" 로 반환.
# 빈 결과 시 "|false|0|0". macOS 엔 tac 없음 → grep|tail 로 해결.
last_entry() {
    local alias=$1 line
    [ -r "$FEED" ] || { echo "|false|0|0"; return; }
    line=$(grep "\"host\":\"$alias\"" "$FEED" | tail -n1)
    if [ -z "$line" ]; then
        echo "|false|0|0"; return
    fi
    printf '%s\n' "$line" | jq -r '
        [(.ts // ""), (.ok | tostring), ((.load1 // 0) | tostring), ((.nproc // 0) | tostring)] | @tsv
    ' 2>/dev/null | tr '\t' '|' || echo "|false|0|0"
}

# ts_iso (UTC) → epoch. 실패 시 0.
ts_to_epoch() {
    local ts=$1
    [ -z "$ts" ] && { echo 0; return; }
    date -u -jf '%Y-%m-%dT%H:%M:%SZ' "$ts" +%s 2>/dev/null || echo 0
}

# load(float) + nproc(int) → free_ci(int).
compute_free_ci() {
    local load1=$1 nproc=$2
    awk -v L="$load1" -v N="$nproc" 'BEGIN{
        f = int(N*100) - int((L+0)*100);
        if (f < 0) f = 0;
        print f
    }'
}

# host-key → (ok load_ci nproc_ci free_ci age_s) space-separated 5-tuple.
# ok=1 means fresh+reachable, 0 otherwise (score=0).
probe_host() {
    local key=$1 alias now entry ts ok_s load1 nproc ep age load_ci nproc_ci free_ci
    alias=$(host_to_alias "$key")
    [ -n "$alias" ] || { echo "0 0 0 0 -1"; return; }
    now=$(date -u +%s)
    entry=$(last_entry "$alias")
    IFS='|' read -r ts ok_s load1 nproc <<< "$entry"
    ep=$(ts_to_epoch "$ts")
    age=$(( ep == 0 ? 99999 : now - ep ))
    if [ "$ok_s" != "true" ] || [ "$ep" = "0" ] || [ "$age" -gt "$FRESH_S" ] || [ "$age" -lt 0 ]; then
        echo "0 0 $((${nproc:-0} * 100)) 0 $age"
        return
    fi
    load_ci=$(awk -v L="$load1" 'BEGIN{printf "%d", (L+0)*100}')
    nproc_ci=$(( nproc * 100 ))
    free_ci=$(compute_free_ci "$load1" "$nproc")
    echo "1 $load_ci $nproc_ci $free_ci $age"
}

# kind + probe 5-tuple + has_gpu → score (int).
score_of() {
    local kind=$1 ok=$2 free_ci=$3 has_gpu=$4
    [ "$ok" != "1" ] && { echo 0; return; }
    case "$kind" in
        compute|heavy) echo "$free_ci" ;;
        gpu)
            if [ "$has_gpu" = "true" ]; then echo "$free_ci"; else echo 0; fi
            ;;
        *) echo 0 ;;
    esac
}

# 각 호스트의 has_gpu 조회
has_gpu() { jq -r --arg h "$1" '.hosts[$h].has_gpu // false' "$REG"; }

# 후보 전체 순회 → 최고점 호스트. strict > 이라 tie 에선 앞 호스트 유지 (candidate_keys 순서).
pick_host() {
    local kind=$1 best="none" best_s=0 key probe ok load_ci nproc_ci free_ci age gpu s
    while IFS= read -r key; do
        [ -z "$key" ] && continue
        probe=$(probe_host "$key")
        read -r ok load_ci nproc_ci free_ci age <<< "$probe"
        gpu=$(has_gpu "$key")
        s=$(score_of "$kind" "$ok" "$free_ci" "$gpu")
        if [ "$s" -gt "$best_s" ]; then
            best=$key
            best_s=$s
        fi
    done < <(candidate_keys)
    echo "$best"
}

# lb_state.json 원자적 기록.
write_state() {
    local kind=$1 chosen=$2 ts tmp hosts_json scores_json key probe ok load_ci nproc_ci free_ci age gpu s
    ts=$(date -u +%FT%TZ)
    hosts_json="{"
    scores_json="{"
    local first=1
    while IFS= read -r key; do
        [ -z "$key" ] && continue
        probe=$(probe_host "$key")
        read -r ok load_ci nproc_ci free_ci age <<< "$probe"
        gpu=$(has_gpu "$key")
        s=$(score_of "$kind" "$ok" "$free_ci" "$gpu")
        [ "$first" = "0" ] && { hosts_json+=","; scores_json+=","; }
        hosts_json+="\"$key\":{\"ok\":$( [ "$ok" = "1" ] && echo true || echo false ),\"load_ci\":$load_ci,\"nproc_ci\":$nproc_ci,\"free_ci\":$free_ci,\"age_s\":$age,\"has_gpu\":$gpu}"
        scores_json+="\"$key\":$s"
        first=0
    done < <(candidate_keys)
    hosts_json+="}"
    scores_json+="}"
    tmp="$STATE.tmp"
    printf '{"ts":"%s","source":"bin/lb.sh","kind":"%s","chosen":"%s","hosts":%s,"scores":%s,"fresh_window_s":%s}\n' \
        "$ts" "$kind" "$chosen" "$hosts_json" "$scores_json" "$FRESH_S" > "$tmp"
    mv -f "$tmp" "$STATE"
}

log_jsonl() {
    local ts=$1 kind=$2 host=$3 alias=$4 rc=$5 ms=$6 cmd=$7 cmd_esc
    cmd_esc=$(printf '%s' "$cmd" | jq -Rs .)
    printf '{"ts":"%s","kind":"%s","host":"%s","alias":"%s","exit":%s,"ms":%s,"cmd":%s}\n' \
        "$ts" "$kind" "$host" "$alias" "$rc" "$ms" "$cmd_esc" >> "$LOG"
}

cmd_pick() {
    local kind=${1:-compute}
    local host
    host=$(pick_host "$kind")
    write_state "$kind" "$host"
    echo "$host"
}

cmd_status() {
    local key probe ok load_ci nproc_ci free_ci age gpu
    printf '%-6s %-6s %-10s %-10s %-10s %-6s %s\n' HOST OK LOAD_CI NPROC_CI FREE_CI AGE GPU
    while IFS= read -r key; do
        [ -z "$key" ] && continue
        probe=$(probe_host "$key")
        read -r ok load_ci nproc_ci free_ci age <<< "$probe"
        gpu=$(has_gpu "$key")
        printf '%-6s %-6s %-10s %-10s %-10s %-6s %s\n' "$key" "$ok" "$load_ci" "$nproc_ci" "$free_ci" "$age" "$gpu"
    done < <(candidate_keys)
    echo "---"
    for k in compute heavy gpu; do
        printf '%-8s → %s\n' "$k" "$(pick_host "$k")"
    done
}

cmd_run() {
    local kind=${1:-}; shift || die "run <kind> <cmd...>"
    [ -n "${kind:-}" ] || die "run: kind 누락"
    [ $# -gt 0 ] || die "run: cmd 누락"
    local host alias ts t0 t1 rc ms
    host=$(cmd_pick "$kind")
    [ "$host" = "none" ] && die "pick=none for kind=$kind (all stale/unfit)"
    alias=$(host_to_alias "$host")
    [ -n "$alias" ] || die "host '$host' ssh_alias 없음"
    ts=$(date -u +%FT%TZ)
    t0=$(date +%s)
    rc=0
    ssh -o ConnectTimeout=5 "$alias" "$*" || rc=$?
    t1=$(date +%s)
    ms=$(( (t1 - t0) * 1000 ))
    log_jsonl "$ts" "$kind" "$host" "$alias" "$rc" "$ms" "$*"
    exit $rc
}

# ── self-test ────────────────────────────────────────────────────
self_test() {
    local fail=0
    echo "lb.sh self-test"

    # 1. compute_free_ci
    local t
    t=$(compute_free_ci 3.12 12); [ "$t" = "888" ] || { echo "  FAIL compute_free_ci(3.12,12)=$t expect 888"; fail=1; }
    t=$(compute_free_ci 0 12);    [ "$t" = "1200" ] || { echo "  FAIL compute_free_ci(0,12)=$t expect 1200"; fail=1; }
    t=$(compute_free_ci 26.40 12); [ "$t" = "0" ] || { echo "  FAIL compute_free_ci(26.40,12)=$t expect 0 (overloaded)"; fail=1; }
    t=$(compute_free_ci 27.00 32); [ "$t" = "500" ] || { echo "  FAIL compute_free_ci(27,32)=$t expect 500"; fail=1; }

    # 2. score_of — kind/gpu 분기
    t=$(score_of compute 1 900 false); [ "$t" = "900" ] || { echo "  FAIL score compute=$t"; fail=1; }
    t=$(score_of compute 0 900 false); [ "$t" = "0" ]   || { echo "  FAIL score dead=$t"; fail=1; }
    t=$(score_of heavy 1 500 true);    [ "$t" = "500" ] || { echo "  FAIL score heavy=$t"; fail=1; }
    t=$(score_of gpu 1 800 true);      [ "$t" = "800" ] || { echo "  FAIL score gpu+gpu=$t"; fail=1; }
    t=$(score_of gpu 1 800 false);     [ "$t" = "0" ]   || { echo "  FAIL score gpu-nogpu=$t"; fail=1; }
    t=$(score_of bogus 1 800 true);    [ "$t" = "0" ]   || { echo "  FAIL score unknown kind=$t"; fail=1; }

    # 3. ts_to_epoch
    t=$(ts_to_epoch "2026-04-14T00:00:00Z"); [ "$t" -gt 1000000000 ] || { echo "  FAIL ts_to_epoch=$t"; fail=1; }
    t=$(ts_to_epoch ""); [ "$t" = "0" ] || { echo "  FAIL ts_to_epoch empty=$t"; fail=1; }

    # 4. candidate_keys — hosts.json 에 kind!=self 3개 이상
    local n
    n=$(candidate_keys | wc -l | tr -d ' ')
    [ "$n" -ge 3 ] || { echo "  FAIL candidate_keys=$n (expect ≥ 3: ubu, ubu2, htz)"; fail=1; }

    # 5. probe_host live — 최소 1 host 가 ok=1 이어야 (remote_load daemon 동작 조건)
    local any_ok=0 key probe ok _r
    while IFS= read -r key; do
        [ -z "$key" ] && continue
        probe=$(probe_host "$key")
        read -r ok _r <<< "$probe"
        [ "$ok" = "1" ] && any_ok=1
    done < <(candidate_keys)
    [ "$any_ok" = "1" ] || echo "  WARN: no host fresh+ok (remote_load daemon 점검 필요)"

    # 6. pick + write_state 통합 — state 파일에 chosen 기록 확인
    local before after
    [ -f "$STATE" ] && before=$(stat -f '%m' "$STATE") || before=0
    cmd_pick compute >/dev/null
    [ -f "$STATE" ] || { echo "  FAIL write_state did not produce $STATE"; fail=1; }
    local chosen
    chosen=$(jq -r '.chosen' "$STATE")
    [ -n "$chosen" ] || { echo "  FAIL state.chosen empty"; fail=1; }

    if [ "$fail" = "0" ]; then
        echo "  ✅ lb.sh self_test PASS"
    else
        echo "  ❌ lb.sh self_test FAIL"
        exit 1
    fi
}

# ── main ────────────────────────────────────────────────────────
case "${1:-}" in
    pick)           shift; cmd_pick "${1:-compute}" ;;
    status)         cmd_status ;;
    run)            shift; cmd_run "$@" ;;
    --self-test)    self_test ;;
    ""|-h|--help)   sed -n '1,30p' "$0" | sed -n '/^# /p' >&2; exit 2 ;;
    *)              die "unknown: $1" ;;
esac
