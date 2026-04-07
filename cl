#!/bin/zsh
# cl2 — Claude Code multi-account launcher v2 (python3-free)
# - rate limit → auto-switch (same terminal)
# - /login → direct CLAUDE_CONFIG_DIR auth (no symlink dance)
# - 순수 awk/grep/sed — python3 제거
#
# ═══════════════════════════════════════════════════════════════
# 절대규칙 (2026-04-07 수렴진화)
#
# 1. zsh 배열은 1-based: ${arr[$i]} (0-based 금지)
# 2. 세션 종료 후 usage 갱신 전 반드시:
#    claude -p "ok" --max-turns 1  → 키체인 토큰 강제 갱신
#    (Claude Code는 세션 유효 시 키체인 업데이트 생략함)
# 3. accounts.json config_dir에 trailing slash 금지
# 4. usage.hexa 호출: $HEXA $AIRGENOME/modules/usage.hexa -- one NAME
# 5. 쿨다운 초기화: echo '{}' > ~/.airgenome/refresh-cooldown.json
# ═══════════════════════════════════════════════════════════════

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

# ─── JSON helpers (pure awk/grep/sed) ───
_json_get() {
    # Extract simple field from JSON file: _json_get file key
    sed 's/"//g;s/.*'"$2"' *: *//;s/[,}].*//' "$1" 2>/dev/null | tr -d ' '
}

_json_config_dir() {
    # Get config_dir for account name from accounts.json
    local name="$1"
    grep -oE '"name":\s*"[^"]*"|"config_dir":\s*"[^"]*"' "$ACCOUNTS_FILE" 2>/dev/null | \
      sed 's/"//g;s/: */|/' | \
      awk -F'|' -v n="$name" '{if($1=="name")cur=$2;if($1=="config_dir"&&cur==n){print $2;exit}}'
}

_json_cache_field() {
    # Get field from usage cache for account: _json_cache_field name field
    sed 's/.*"'"$1"'":{//;s/}.*//' "$USAGE_CACHE" 2>/dev/null | \
      grep -o '"'"$2"'":[^,}]*' | sed 's/[^:]*://;s/"//g'
}

# ─── login ───
if [[ "$1" == "login" ]]; then
    TARGET="${2:-}"
    if [ -z "$TARGET" ]; then
        TARGET=$(_json_get "$STATE_FILE" "active")
    fi
    if [ -z "$TARGET" ]; then
        echo "  ✗ 계정 지정 필요: cl2 login claude3"
        exit 1
    fi
    TARGET_DIR=$(_json_config_dir "$TARGET")
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
if [[ "$1" == "pick" || "$1" == "add" || "$1" == "remove" || "$1" == "help" ]]; then
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
                local exists=$(grep -c "\"name\":\"$name\"" "$ACCOUNTS_FILE" 2>/dev/null)
                if [ "${exists:-0}" -eq 0 ]; then
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
    # List accounts: name|config_dir|removed, find lowest week usage
    grep -oE '"name":\s*"[^"]*"|"config_dir":\s*"[^"]*"|"removed":\s*[a-z]+' "$ACCOUNTS_FILE" 2>/dev/null | \
      sed 's/"//g;s/: */|/' | \
      awk -F'|' '{if($1=="name")n=$2;if($1=="removed"){r=$2;if(n!="")print n"|"r;n=""}}' | \
      while read entry; do
        local n="${entry%%|*}"
        local r="${entry##*|}"
        [ "$r" = "true" ] && continue
        [ "$n" = "$current" ] && continue
        local w=$(_json_cache_field "$n" "week_all_pct")
        local wi=$(echo "${w:-999}" | awk '{printf "%.0f",$1+0}')
        [ "$wi" -ge 100 ] 2>/dev/null && continue
        echo "$wi|$n"
      done | sort -n | head -1 | cut -d'|' -f2
}

get_config_dir() {
    _json_config_dir "$1"
}

# ─── Cleanup ───
cleanup() {
    [ -n "$FSWATCH_PID" ] && kill "$FSWATCH_PID" 2>/dev/null
    rm -f "$LOGFILE"
}
trap cleanup EXIT INT TERM

# ─── Main ───

cd "$AIRGENOME"
# LAUNCH 마커를 파일로 전달 (대시보드+키입력은 tty 직접)
LAUNCH_MARKER="/tmp/cl-launch-$$"
rm -f "$LAUNCH_MARKER"
export CL_LAUNCH_MARKER="$LAUNCH_MARKER"
$HEXA run modules/cl.hexa "$@" 2>&1 | while IFS= read -r line; do
    case "$line" in
        LAUNCH:*) echo "${line#LAUNCH:}" > "$LAUNCH_MARKER" ;;
        *) printf '%s\n' "$line" ;;
    esac
done
LAUNCH_DIR=""
[ -f "$LAUNCH_MARKER" ] && LAUNCH_DIR=$(cat "$LAUNCH_MARKER")
rm -f "$LAUNCH_MARKER"
cd "$ORIG_DIR"

if [ -z "$LAUNCH_DIR" ]; then
    exit 0
fi

start_account_watcher

CURRENT_DIR="$LAUNCH_DIR"
CURRENT_NAME=$(_json_get "$STATE_FILE" "active")
[ -z "$CURRENT_NAME" ] && CURRENT_NAME="unknown"
SWITCH_COUNT=0
# Count non-removed accounts for max switches
MAX_SWITCHES=$(grep -oE '"removed":[a-z]+' "$ACCOUNTS_FILE" 2>/dev/null | grep -c 'false')
MAX_SWITCHES=$((MAX_SWITCHES > 1 ? MAX_SWITCHES - 1 : 1))

while true; do
    echo ""
    echo "  ▶ Claude Code 시작 [$CURRENT_NAME]"
    echo "  ─────────────────────────────────────"

    export CLAUDE_CONFIG_DIR="$CURRENT_DIR"

    ~/.local/bin/claude
    EXIT_CODE=$?

    # 세션 종료 후 해당 계정 usage 즉시 갱신
    # 1) 키체인 토큰 갱신: claude -p 로 짧은 실행 → 키체인에 새 토큰 저장
    # 2) usage API 호출
    echo ""
    echo "  ⬡ $CURRENT_NAME usage 갱신 중..."
    echo '{}' > ~/.airgenome/refresh-cooldown.json

    # Step 1: 키체인 토큰이 만료됐으면 CLI로 강제 갱신
    CLAUDE_CONFIG_DIR="$CURRENT_DIR" ~/.local/bin/claude -p "ok" --max-turns 1 >/dev/null 2>&1

    # Step 2: usage API 호출
    _result=$(cd "$AIRGENOME" && $HEXA $AIRGENOME/modules/usage.hexa -- one "$CURRENT_NAME" 2>&1)
    if echo "$_result" | grep -q '✓'; then
        echo "$_result" | tail -2
    else
        echo "    ✗ 갱신 실패"
    fi

    LATEST_JSONL=$(ls -t "${CURRENT_DIR}projects/"*"/"{sessions,}/*.jsonl 2>/dev/null | head -1)
    if [ -n "$LATEST_JSONL" ]; then
        tail -50 "$LATEST_JSONL" > "$LOGFILE" 2>/dev/null
    fi

    if check_rate_limit "$LOGFILE" && [ $SWITCH_COUNT -lt $MAX_SWITCHES ]; then
        echo ""
        echo "  ⚠ Rate limit 감지! 자동 계정 전환 중..."

        NEXT=$(pick_next_account "$CURRENT_NAME")

        if [ -z "$NEXT" ] || [ "$NEXT" = "none" ]; then
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

        local s_pct=$(_json_cache_field "$NEXT" "session_pct")
        local w_pct=$(_json_cache_field "$NEXT" "week_all_pct")
        [ -z "$s_pct" ] && s_pct="?"
        [ -z "$w_pct" ] && w_pct="?"

        echo "  ⬡ $NEXT  session=${s_pct}%  week=${w_pct}%"
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
echo "  ⬡ cl 종료 (전환 ${SWITCH_COUNT}회)"
