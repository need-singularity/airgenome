#!/bin/zsh
# cl — Claude Code multi-account launcher (airgenome)
# #2: rate limit → auto-switch (same terminal)
# #4: /login → fswatch auto-detect new accounts

# iTerm2 프로파일 커맨드에서도 동작하도록 login 환경 수동 로드
[ -f ~/.zprofile ] && source ~/.zprofile
[ -f ~/.zshrc ] && source ~/.zshrc

ORIG_DIR="$(pwd)"
HEXA=~/Dev/hexa-lang/hexa
AIRGENOME=~/Dev/airgenome
ACCOUNTS_FILE=~/.airgenome/accounts.json
STATE_FILE=~/.airgenome/cl-state.json
USAGE_CACHE=~/.airgenome/usage-cache.json
LOGFILE="${TMPDIR:-/tmp}/cl-claude-output.log"
FSWATCH_PID=""

# Rate limit patterns
RATE_PATTERNS=(
    "rate.limit"
    "limit.reached"
    "usage.cap"
    "rate_limit_error"
    "Too.many.requests"
    "overloaded_error"
    "over.your.usage.limit"
    "exceeded.*limit"
)

# ─── Subcommands (pass through to hexa) ───
if [[ "$1" == "status" || "$1" == "pick" || "$1" == "add" || "$1" == "remove" || "$1" == "help" ]]; then
    cd "$AIRGENOME"
    $HEXA run modules/cl.hexa "$@" 2>&1
    cd "$ORIG_DIR"
    exit 0
fi

# ─── #4: fswatch — 새 계정 자동 감지 ───
start_account_watcher() {
    # 현재 등록된 디렉토리 스냅샷
    local known_dirs="${TMPDIR:-/tmp}/cl-known-dirs.txt"
    ls -d ~/.claude-claude*/ 2>/dev/null | sort > "$known_dirs"

    (
        # ~/.claude-* 디렉토리에 새 .claude.json 생성 감시
        fswatch -r --event Created --event Updated ~/.claude-claude*/.claude.json 2>/dev/null | while read changed_file; do
            local dir=$(dirname "$changed_file")
            local name=$(basename "$dir" | sed 's/^\.//' | sed 's/^claude-//')

            # accounts.json에 이미 있는지 확인
            if [ -f "$ACCOUNTS_FILE" ]; then
                local exists=$(python3 -c "import json;d=json.load(open('$ACCOUNTS_FILE'));print('yes' if any(a['name']=='$name' for a in d['accounts']) else 'no')" 2>/dev/null)
                if [ "$exists" = "no" ]; then
                    echo ""
                    echo "  ⬡ 🆕 새 계정 감지: $name ($dir)"
                    cd "$AIRGENOME"
                    $HEXA run modules/cl.hexa add "$name" "$dir/" 2>&1
                    cd "$ORIG_DIR"
                fi
            fi
        done
    ) &
    FSWATCH_PID=$!
}

# ─── #2: Rate limit 감지 ───
check_rate_limit() {
    local logfile="$1"
    [ ! -f "$logfile" ] && return 1

    # 마지막 200줄에서 rate limit 패턴 검색
    local tail_content=$(tail -200 "$logfile" 2>/dev/null)
    for pattern in "${RATE_PATTERNS[@]}"; do
        if echo "$tail_content" | grep -qiE "$pattern" 2>/dev/null; then
            return 0  # rate limit detected
        fi
    done
    return 1  # no rate limit
}

pick_next_account() {
    local current="$1"
    # 현재 계정 제외하고 best 선택
    python3 -c "
import json, os
try:
    accounts = json.load(open(os.path.expanduser('$ACCOUNTS_FILE')))['accounts']
    try: usage = json.load(open(os.path.expanduser('$USAGE_CACHE')))
    except: usage = {}
    best = None; best_week = 999
    for a in accounts:
        if a.get('removed'): continue
        n = a['name']
        if n == '$current': continue
        u = usage.get(n, {})
        w = u.get('week_all_pct', 999)
        try: w = float(w)
        except: w = 999
        if w >= 100: continue
        if w < best_week: best = n; best_week = w
    if best: print(f'{best}')
    else: print('none')
except: print('none')
" 2>/dev/null
}

get_config_dir() {
    local name="$1"
    python3 -c "
import json, os
d = json.load(open(os.path.expanduser('$ACCOUNTS_FILE')))
for a in d['accounts']:
    if a['name'] == '$name':
        print(a['config_dir'])
        break
" 2>/dev/null
}

backup_and_launch() {
    local name="$1"
    local config_dir="$2"

    # 오염 방지: 백업
    if [ -f "${config_dir}.claude.json" ]; then
        cp "${config_dir}.claude.json" "${config_dir}.claude.json.bak"
    fi

    # 활성 상태 저장
    mkdir -p ~/.airgenome
    echo "{\"active\":\"$name\"}" > "$STATE_FILE"

    # Usage 정보
    local usage_info=$(python3 -c "
import json, os
try:
    d = json.load(open(os.path.expanduser('$USAGE_CACHE')))
    e = d.get('$name', {})
    s = e.get('session_pct', '?')
    w = e.get('week_all_pct', '?')
    print(f'session={s}%  week={w}%')
except: print('session=?%  week=?%')
" 2>/dev/null)

    echo "  ⬡ $name  $usage_info"
    echo "    $config_dir"
}

check_contamination() {
    local config_dir="$1"
    local json_path="${config_dir}.claude.json"
    local bak_path="${config_dir}.claude.json.bak"

    if [ -f "$bak_path" ] && [ -f "$json_path" ]; then
        local orig_uuid=$(python3 -c "import json;print(json.load(open('$bak_path')).get('oauthAccount',{}).get('accountUuid',''))" 2>/dev/null)
        local curr_uuid=$(python3 -c "import json;print(json.load(open('$json_path')).get('oauthAccount',{}).get('accountUuid',''))" 2>/dev/null)

        if [ -n "$orig_uuid" ] && [ "$orig_uuid" != "$curr_uuid" ]; then
            echo "  ⚠ 오염 감지! UUID 변경: $orig_uuid → $curr_uuid"
            cp "$bak_path" "$json_path"
            echo "  ✓ 백업에서 복원 완료"
        fi
    fi
}

# ─── Cleanup ───
cleanup() {
    [ -n "$FSWATCH_PID" ] && kill "$FSWATCH_PID" 2>/dev/null
    rm -f "$LOGFILE" "${TMPDIR:-/tmp}/cl-known-dirs.txt"
}
trap cleanup EXIT INT TERM

# ─── Main: auto-switch loop ───

# Usage 캐시 갱신
cd "$AIRGENOME"
$HEXA run modules/usage.hexa auto 2>/dev/null
cd "$ORIG_DIR"

# 첫 계정 선택
cd "$AIRGENOME"
OUTPUT=$($HEXA run modules/cl.hexa "$@" 2>&1)
LAUNCH_DIR=$(echo "$OUTPUT" | grep '^LAUNCH:' | sed 's/^LAUNCH://')
cd "$ORIG_DIR"

echo "$OUTPUT" | grep -v '^LAUNCH:'

if [ -z "$LAUNCH_DIR" ]; then
    echo ""
    echo "[cl] claude 실행 실패 — LAUNCH 마커 없음"
    echo "[cl] 아무 키나 누르면 종료..."
    read -r _
    exit 1
fi

# fswatch 시작 (#4)
start_account_watcher

# ─── Rate limit auto-switch loop (#2) ───
CURRENT_DIR="$LAUNCH_DIR"
CURRENT_NAME=$(python3 -c "
import json, os
try:
    d = json.load(open(os.path.expanduser('$STATE_FILE')))
    print(d['active'])
except: print('unknown')
" 2>/dev/null)
SWITCH_COUNT=0
MAX_SWITCHES=9  # 10개 계정 - 1

while true; do
    echo ""
    echo "  ▶ Claude Code 시작 [$CURRENT_NAME]"
    echo "  ─────────────────────────────────────"

    # script로 출력 캡처하면서 실행 (같은 화면)
    export CLAUDE_CONFIG_DIR="$CURRENT_DIR"
    > "$LOGFILE"  # clear log

    # script -q: quiet, 출력 캡처 + 화면 표시 동시
    script -q "$LOGFILE" ~/.local/bin/claude
    EXIT_CODE=$?

    # 오염 체크
    check_contamination "$CURRENT_DIR"

    # Rate limit 확인
    if check_rate_limit "$LOGFILE" && [ $SWITCH_COUNT -lt $MAX_SWITCHES ]; then
        echo ""
        echo "  ⚠ Rate limit 감지! 자동 계정 전환 중..."

        # Usage 캐시 갱신
        cd "$AIRGENOME"
        $HEXA run modules/usage.hexa auto 2>/dev/null
        cd "$ORIG_DIR"

        NEXT=$(pick_next_account "$CURRENT_NAME")

        if [ "$NEXT" = "none" ]; then
            echo "  ✗ 사용 가능한 계정 없음 — 모두 소진"
            echo ""
            cd "$AIRGENOME"
            $HEXA run modules/cl.hexa status 2>&1
            cd "$ORIG_DIR"
            break
        fi

        NEXT_DIR=$(get_config_dir "$NEXT")
        if [ -z "$NEXT_DIR" ]; then
            echo "  ✗ 계정 디렉토리 없음: $NEXT"
            break
        fi

        backup_and_launch "$NEXT" "$NEXT_DIR"
        CURRENT_DIR="$NEXT_DIR"
        CURRENT_NAME="$NEXT"
        SWITCH_COUNT=$((SWITCH_COUNT + 1))

        echo "  ↻ 전환 $SWITCH_COUNT/$MAX_SWITCHES → $CURRENT_NAME"
        echo ""
        sleep 1
        # loop continues → re-launch claude
    else
        # 정상 종료 or 최대 전환 초과
        if [ $SWITCH_COUNT -ge $MAX_SWITCHES ]; then
            echo "  ✗ 최대 전환 횟수 도달 ($MAX_SWITCHES)"
        fi
        break
    fi
done

echo ""
echo "  ⬡ cl 종료 (전환 ${SWITCH_COUNT}회)"
