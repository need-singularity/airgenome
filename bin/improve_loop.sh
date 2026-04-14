#!/usr/bin/env bash
# bin/improve_loop.sh — airgenome Claude Code CLI 개선 루프 MVP
#
# 목적: idle 한 Claude Code 계정을 airgenome 자가개선에 활용.
#       큐에서 pending task 하나씩 꺼내 `claude -p` 로 spawn, 결과 capture.
#
# 큐 파일: ~/.airgenome/improve_queue.jsonl
#   각 라인 = 하나의 task JSON:
#     {"id": "2026-04-14T09:50:00Z-deadbeef",
#      "ts": "2026-04-14T09:50:00Z",
#      "task": "...한글로 된 태스크 설명...",
#      "status": "pending|in_progress|done|fail",
#      "started_at": null, "finished_at": null,
#      "result_path": null, "exit": null}
#
# 동작:
#   1) improve_queue.jsonl scan → 첫 pending 항목 선택
#   2) status=in_progress 로 마킹 + started_at
#   3) claude -p "<task>" --max-turns 10 --cwd ~/Dev/airgenome 실행
#      stdout → ~/.airgenome/improve_results/<id>.out
#   4) 완료시 exit code 기록 + status=done|fail + finished_at + result_path
#
# 사용:
#   bin/improve_loop.sh run         # 1개 pending → 1개 spawn → 동기 대기
#   bin/improve_loop.sh run-bg      # 1개 pending → 1개 spawn → 백그라운드
#   bin/improve_loop.sh add "task"  # 새 task 큐에 push
#   bin/improve_loop.sh list        # 큐 상태 출력
#   bin/improve_loop.sh fill        # 기본 태스크 세트를 큐에 로드 (bootstrap)

set -euo pipefail

Q="${HOME}/.airgenome/improve_queue.jsonl"
RESULTS="${HOME}/.airgenome/improve_results"
LOG="${HOME}/.airgenome/improve_loop.log"
WORKDIR="${HOME}/Dev/airgenome"

mkdir -p "$RESULTS" "$(dirname "$Q")"
touch "$Q"

now_iso() { date -u +%Y-%m-%dT%H:%M:%SZ; }
gen_id()  { echo "$(now_iso)-$(openssl rand -hex 4)"; }
log()     { echo "[$(now_iso)] $*" | tee -a "$LOG"; }

cmd_add() {
    local task="${1:-}"
    [ -z "$task" ] && { echo "usage: $0 add \"task description\"" >&2; exit 1; }
    local id; id=$(gen_id)
    local ts; ts=$(now_iso)
    jq -nc --arg id "$id" --arg ts "$ts" --arg task "$task" '{
        id: $id, ts: $ts, task: $task,
        status: "pending", started_at: null, finished_at: null,
        result_path: null, exit: null
    }' >> "$Q"
    echo "queued: $id"
}

cmd_list() {
    [ -s "$Q" ] || { echo "(empty queue)"; return; }
    local pending done_count fail_count in_progress
    pending=$(grep -c '"status":"pending"' "$Q" 2>/dev/null || echo 0)
    in_progress=$(grep -c '"status":"in_progress"' "$Q" 2>/dev/null || echo 0)
    done_count=$(grep -c '"status":"done"' "$Q" 2>/dev/null || echo 0)
    fail_count=$(grep -c '"status":"fail"' "$Q" 2>/dev/null || echo 0)
    echo "queue: $(wc -l < "$Q") total  pending=$pending in_progress=$in_progress done=$done_count fail=$fail_count"
    echo "--- recent 5 ---"
    tail -5 "$Q" | jq -rc '.status + "  " + .id + "  " + (.task[0:60])'
}

# 기본 bootstrap 태스크 — 모두 READ-ONLY 안전 분석 태스크
cmd_fill() {
    local tasks=(
        "bin/menubar.hexa 를 읽고, 아직 config SSOT 에 들어가지 않은 하드코드 값(매직 넘버/문자열)을 모두 찾아 목록으로 출력해줘. 파일 경로와 라인 번호 포함. 수정은 하지 말고 목록만."
        "bin/test_menubar.sh 하네스가 현재 커버하지 못하는 영역을 3가지 찾아서 각각 어떻게 보완할지 간단히 제안해줘. 코드는 수정하지 말고 설명만."
        "modules/ 디렉토리의 hexa 파일 중 최근 1개월 내 어떤 다른 파일에서도 참조되지 않는 (use 문으로 import 안 되는) 파일들을 찾아줘. 찾은 파일 목록만 출력, 삭제/수정 금지."
        "forge 디렉토리의 링 파일들 (genomes*.ring) 의 현재 크기와 mtime 을 조사해서, 갱신이 멈춘 것이 있는지 보고해줘. 수정 금지."
        "shared/config/roadmap/airgenome.json 의 milestones 섹션을 읽고, 현재 상태(done/wip/pending) 통계와 다음 예상 milestone 을 1개 제안해줘."
    )
    for t in "${tasks[@]}"; do
        cmd_add "$t"
    done
    echo "filled: ${#tasks[@]} tasks"
    cmd_list
}

cmd_run() {
    # 첫 pending 라인 찾기 — 없으면 auto-refill 후 재시도
    local line line_num
    line_num=$(grep -n '"status":"pending"' "$Q" 2>/dev/null | head -1 | cut -d: -f1 || true)
    if [ -z "$line_num" ]; then
        log "⚙ queue empty — auto-refill"
        cmd_fill >/dev/null
        line_num=$(grep -n '"status":"pending"' "$Q" 2>/dev/null | head -1 | cut -d: -f1 || true)
        [ -z "$line_num" ] && { echo "(refill failed, still empty)"; return 0; }
    fi

    line=$(sed -n "${line_num}p" "$Q")
    local id task
    id=$(echo "$line" | jq -r '.id')
    task=$(echo "$line" | jq -r '.task')

    log "▶ spawning claude -p for task: $id"
    log "  task: ${task:0:100}..."

    # in_progress 마킹 (파일 덮어쓰기 — atomic via tmp)
    local started; started=$(now_iso)
    local result_path="$RESULTS/${id}.out"
    local tmp="$Q.tmp.$$"
    awk -v n="$line_num" -v started="$started" -v rp="$result_path" '
        NR==n {
            gsub(/"status":"pending"/, "\"status\":\"in_progress\"")
            gsub(/"started_at":null/, "\"started_at\":\"" started "\"")
            gsub(/"result_path":null/, "\"result_path\":\"" rp "\"")
        }
        { print }
    ' "$Q" > "$tmp" && mv "$tmp" "$Q"

    # spawn claude -p — 짧은 실행, 읽기 전용 태스크 의도
    local exit_code=0
    (cd "$WORKDIR" && claude -p "$task" --max-turns 10) > "$result_path" 2>&1 || exit_code=$?

    local finished; finished=$(now_iso)
    local final_status="done"
    [ "$exit_code" -ne 0 ] && final_status="fail"

    # 결과 마킹
    local tmp2="$Q.tmp2.$$"
    awk -v n="$line_num" -v fin="$finished" -v stat="$final_status" -v ec="$exit_code" '
        NR==n {
            gsub(/"status":"in_progress"/, "\"status\":\"" stat "\"")
            gsub(/"finished_at":null/, "\"finished_at\":\"" fin "\"")
            gsub(/"exit":null/, "\"exit\":" ec)
        }
        { print }
    ' "$Q" > "$tmp2" && mv "$tmp2" "$Q"

    log "✓ $id → $final_status (exit $exit_code, $(wc -l < "$result_path") lines captured)"
    echo "--- result head (${result_path}) ---"
    head -20 "$result_path"
}

cmd_run_bg() {
    nohup "$0" run > /dev/null 2>&1 &
    echo "backgrounded: pid=$!"
}

case "${1:-}" in
    add)     shift; cmd_add "$@" ;;
    list)    cmd_list ;;
    fill)    cmd_fill ;;
    run)     cmd_run ;;
    run-bg)  cmd_run_bg ;;
    *)
        echo "usage: $0 {add|list|fill|run|run-bg}"
        echo "  fill    — bootstrap 기본 5개 read-only 분석 태스크"
        echo "  run     — pending 1개 → claude -p 동기 실행"
        echo "  run-bg  — pending 1개 → claude -p 백그라운드"
        echo "  add \"task\" — 큐에 새 태스크 추가"
        echo "  list    — 큐 상태"
        exit 1
        ;;
esac
