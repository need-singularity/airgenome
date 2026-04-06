#!/bin/zsh
# cl2 — Claude Code multi-account launcher v2 (contamination-free)
# - rate limit → auto-switch (same terminal)
# - /login → direct CLAUDE_CONFIG_DIR auth (no symlink dance)
# - no backup/restore/contamination checks

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

# ─── login ───
if [[ "$1" == "login" ]]; then
    TARGET="${2:-}"
    if [ -z "$TARGET" ]; then
        TARGET=$(python3 -c "
import json, os
try:
    d = json.load(open(os.path.expanduser('$STATE_FILE')))
    print(d['active'])
except: print('')
" 2>/dev/null)
    fi
    if [ -z "$TARGET" ]; then
        echo "  ✗ 계정 지정 필요: cl2 login claude3"
        exit 1
    fi
    TARGET_DIR=$(python3 -c "
import json, os
d = json.load(open(os.path.expanduser('$ACCOUNTS_FILE')))
for a in d['accounts']:
    if a['name'] == '$TARGET':
        print(a['config_dir'])
        break
" 2>/dev/null)
    if [ -z "$TARGET_DIR" ]; then
        echo "  ✗ 계정 없음: $TARGET"
        exit 1
    fi

    echo "  ⬡ 로그인: $TARGET ($TARGET_DIR)"
    echo ""
    export CLAUDE_CONFIG_DIR="$TARGET_DIR"
    ~/.local/bin/claude auth login
    exit $?
fi

# ─── Subcommands ───
if [[ "$1" == "status" || "$1" == "u" || "$1" == "-u" || "$1" == "pick" || "$1" == "add" || "$1" == "remove" || "$1" == "help" ]]; then
    # -u → u 변환
    [[ "$1" == "-u" ]] && set -- "u" "${@:2}"
    cd "$AIRGENOME"
    $HEXA run modules/cl.hexa "$@" 2>&1
    cd "$ORIG_DIR"
    exit 0
fi

# ─── fswatch ───
start_account_watcher() {
    (
        fswatch -r --event Created --event Updated ~/.claude-claude*/.claude.json 2>/dev/null | while read changed_file; do
            local dir=$(dirname "$changed_file")
            local name=$(basename "$dir" | sed 's/^\.//' | sed 's/^claude-//')
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

# ─── Rate limit ───
check_rate_limit() {
    local logfile="$1"
    [ ! -f "$logfile" ] && return 1
    local tail_content=$(tail -200 "$logfile" 2>/dev/null)
    for pattern in "${RATE_PATTERNS[@]}"; do
        if echo "$tail_content" | grep -qiE "$pattern" 2>/dev/null; then
            return 0
        fi
    done
    return 1
}

pick_next_account() {
    local current="$1"
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

# ─── Cleanup ───
cleanup() {
    [ -n "$FSWATCH_PID" ] && kill "$FSWATCH_PID" 2>/dev/null
    rm -f "$LOGFILE"
}
trap cleanup EXIT INT TERM

# ─── Main ───

(cd "$AIRGENOME" && $HEXA run modules/usage.hexa auto >/dev/null 2>&1 &)

cd "$AIRGENOME"
OUTPUT=$($HEXA run modules/cl.hexa "$@" 2>&1)
LAUNCH_DIR=$(echo "$OUTPUT" | grep '^LAUNCH:' | sed 's/^LAUNCH://')
cd "$ORIG_DIR"

echo "$OUTPUT" | grep -v '^LAUNCH:'

if [ -z "$LAUNCH_DIR" ]; then
    echo ""
    echo "[cl2] claude 실행 실패 — LAUNCH 마커 없음"
    echo "[cl2] 아무 키나 누르면 종료..."
    read -r _
    exit 1
fi

start_account_watcher

CURRENT_DIR="$LAUNCH_DIR"
CURRENT_NAME=$(python3 -c "
import json, os
try:
    d = json.load(open(os.path.expanduser('$STATE_FILE')))
    print(d['active'])
except: print('unknown')
" 2>/dev/null)
SWITCH_COUNT=0
MAX_SWITCHES=$(python3 -c "
import json, os
try:
    d = json.load(open(os.path.expanduser('$ACCOUNTS_FILE')))
    active = [a for a in d['accounts'] if not a.get('removed')]
    print(max(len(active) - 1, 1))
except: print(9)
" 2>/dev/null)

while true; do
    echo ""
    echo "  ▶ Claude Code 시작 [$CURRENT_NAME]"
    echo "  ─────────────────────────────────────"

    export CLAUDE_CONFIG_DIR="$CURRENT_DIR"
    ~/.local/bin/claude
    EXIT_CODE=$?

    LATEST_JSONL=$(ls -t "${CURRENT_DIR}projects/"*"/"{sessions,}/*.jsonl 2>/dev/null | head -1)
    if [ -n "$LATEST_JSONL" ]; then
        tail -50 "$LATEST_JSONL" > "$LOGFILE" 2>/dev/null
    fi

    if check_rate_limit "$LOGFILE" && [ $SWITCH_COUNT -lt $MAX_SWITCHES ]; then
        echo ""
        echo "  ⚠ Rate limit 감지! 자동 계정 전환 중..."

        (cd "$AIRGENOME" && $HEXA run modules/usage.hexa auto >/dev/null 2>&1 &)

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

        mkdir -p ~/.airgenome
        echo "{\"active\":\"$NEXT\"}" > "$STATE_FILE"

        local usage_info=$(python3 -c "
import json, os
try:
    d = json.load(open(os.path.expanduser('$USAGE_CACHE')))
    e = d.get('$NEXT', {})
    s = e.get('session_pct', '?')
    w = e.get('week_all_pct', '?')
    print(f'session={s}%  week={w}%')
except: print('session=?%  week=?%')
" 2>/dev/null)

        echo "  ⬡ $NEXT  $usage_info"
        echo "    $NEXT_DIR"

        CURRENT_DIR="$NEXT_DIR"
        CURRENT_NAME="$NEXT"
        SWITCH_COUNT=$((SWITCH_COUNT + 1))

        echo "  ↻ 전환 $SWITCH_COUNT/$MAX_SWITCHES → $CURRENT_NAME"
        echo ""
        sleep 1
    else
        if [ $SWITCH_COUNT -ge $MAX_SWITCHES ]; then
            echo "  ✗ 최대 전환 횟수 도달 ($MAX_SWITCHES)"
        fi
        break
    fi
done

echo ""
echo "  ⬡ cl2 종료 (전환 ${SWITCH_COUNT}회)"
