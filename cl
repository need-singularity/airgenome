#!/bin/sh
# cl — Claude Code multi-account launcher via airgenome gate
# 폐기: 이 파일 + ~/.zshrc 한 줄 삭제하면 끝
#
# Usage:
#   cl              최적 계정으로 claude 실행
#   cl status       계정 현황 (6축 hexagon)
#   cl pick <N>     계정 N 지정 실행
#   cl add          새 계정 로그인
#   cl remove <N>   계정 폐기
#   cl help         도움말

set -e

CONF="$HOME/.airgenome"
ACCOUNTS="$CONF/accounts.json"
STATE="$CONF/cl-state.json"
USAGE="$HOME/.ccmon-usage-cache.json"
PY="/opt/homebrew/bin/python3.12"
CLAUDE="$(command -v claude 2>/dev/null || echo "$HOME/.local/bin/claude")"

mkdir -p "$CONF"

# ── 색상 ──
R='\033[31m' G='\033[32m' Y='\033[33m' C='\033[36m' W='\033[1;37m' D='\033[90m' N='\033[0m'

# ══════════════════════════════════════════════════════════════════════
#  BOOTSTRAP: 계정 자동 발견
# ══════════════════════════════════════════════════════════════════════
bootstrap() {
    [ -f "$ACCOUNTS" ] && return 0

    printf "${C}[cl]${N} 계정 스캔 중...\n"
    $PY - << 'PYEOF'
import json, os, glob

accounts = []
for d in sorted(glob.glob(os.path.expanduser("~/.claude-claude*/"))):
    cj = os.path.join(d, ".claude.json")
    if not os.path.isfile(cj): continue
    name = os.path.basename(d.rstrip("/")).replace(".", "").replace("claude-", "")
    accounts.append({"name": name, "dir": d, "removed": False})

out = os.path.expanduser("~/.airgenome/accounts.json")
os.makedirs(os.path.dirname(out), exist_ok=True)
json.dump({"accounts": accounts}, open(out, "w"), indent=2)
print(f"  {len(accounts)} accounts found")
PYEOF
}

# ══════════════════════════════════════════════════════════════════════
#  HEXAGON GATE: 6축 projection per account
# ══════════════════════════════════════════════════════════════════════
#  cpu=session%  ram=week%  gpu=cooldown  npu=errors  power=contamination  io=active

gate_project_all() {
    $PY - << 'PYEOF'
import json, os

accts = json.load(open(os.path.expanduser("~/.airgenome/accounts.json")))["accounts"]
usage = {}
try: usage = json.load(open(os.path.expanduser("~/.ccmon-usage-cache.json")))
except: pass

state = {}
try: state = json.load(open(os.path.expanduser("~/.airgenome/cl-state.json")))
except: pass
active = state.get("active", "")

for a in accts:
    if a.get("removed"): continue
    n = a["name"]
    u = usage.get(n, {})

    # 6-axis projection
    cpu = float(u.get("session_pct", 0) or 0)      # session usage
    ram = float(u.get("week_all_pct", 0) or 0)      # weekly usage
    gpu = float(u.get("cooldown_min", 0) or 0)      # cooldown
    npu = 1.0 if u.get("error") and u["error"] != "None" else 0.0
    power = 0.0                                       # contamination (0=clean)
    io = 1.0 if n == active else 0.0                  # active session

    # genome score: lower = better
    score = cpu * 0.3 + ram * 0.5 + gpu * 0.1 + npu * 100 + power * 100

    # status
    status = ""
    if n == active: status = "● ACTIVE"
    if ram >= 100: status = "✗ EXHAUSTED"
    elif npu > 0: status = "✗ ERROR"
    elif ram >= 80: status = "⚠ HIGH"

    # bar visualization
    def bar(v, w=10):
        f = int(min(v/100, 1.0) * w)
        return "█" * f + "░" * (w - f)

    print(f"{n}|{cpu:.0f}|{ram:.0f}|{gpu:.0f}|{npu:.0f}|{power:.0f}|{io:.0f}|{score:.1f}|{status}|{bar(cpu)}|{bar(ram)}|{a['dir']}")
PYEOF
}

# ══════════════════════════════════════════════════════════════════════
#  PICK BEST: gate score 최저 계정 선택
# ══════════════════════════════════════════════════════════════════════
pick_best() {
    $PY - << 'PYEOF'
import json, os

accts = json.load(open(os.path.expanduser("~/.airgenome/accounts.json")))["accounts"]
usage = {}
try: usage = json.load(open(os.path.expanduser("~/.ccmon-usage-cache.json")))
except: pass

best = None; best_score = 9999
for a in accts:
    if a.get("removed"): continue
    n = a["name"]
    u = usage.get(n, {})
    cpu = float(u.get("session_pct", 0) or 0)
    ram = float(u.get("week_all_pct", 0) or 0)
    npu = 1.0 if u.get("error") and u["error"] != "None" else 0.0
    if ram >= 100: continue
    if npu > 0: continue
    score = cpu * 0.3 + ram * 0.5 + npu * 100
    if score < best_score:
        best_score = score; best = n
if best: print(f"{best}|{best_score:.1f}")
else:
    # fallback: first non-removed
    for a in accts:
        if not a.get("removed"):
            print(f"{a['name']}|999"); break
PYEOF
}

# ══════════════════════════════════════════════════════════════════════
#  STATUS: hexagon display
# ══════════════════════════════════════════════════════════════════════
show_status() {
    bootstrap
    printf "\n"
    printf "  ${W}⬡ Claude Code Hexagon Gate${N}\n"
    printf "  ${D}  cpu=session  ram=week  gpu=cooldown  npu=error${N}\n\n"
    printf "  ${W}%-10s  %4s  %4s  %-10s  %-10s  %6s  %s${N}\n" \
        "ACCOUNT" "SES%" "WK%" "SESSION" "WEEKLY" "SCORE" "STATUS"
    printf "  ${D}──────────  ────  ────  ──────────  ──────────  ──────  ──────────${N}\n"

    gate_project_all | while IFS='|' read -r name cpu ram gpu npu power io score status cpu_bar ram_bar dir; do
        color="$N"
        case "$status" in
            *EXHAUSTED*) color="$R" ;;
            *ERROR*)     color="$R" ;;
            *HIGH*)      color="$Y" ;;
            *ACTIVE*)    color="$G" ;;
        esac
        printf "  ${color}%-10s  %3s%%  %3s%%  %s  %s  %6s  %s${N}\n" \
            "$name" "$cpu" "$ram" "$cpu_bar" "$ram_bar" "$score" "$status"
    done
    printf "\n"
}

# ══════════════════════════════════════════════════════════════════════
#  오염 방지 (Contamination Guard)
# ══════════════════════════════════════════════════════════════════════
guard_backup() {
    local cj="$1/.claude.json"
    [ -f "$cj" ] && cp "$cj" "$cj.bak.$$"
}

guard_check() {
    local dir="$1" bak="$dir/.claude.json.bak.$$"
    [ -f "$bak" ] || return 0
    local orig new
    orig=$($PY -c "import json;print(json.load(open('$bak')).get('oauthAccount',{}).get('accountUuid',''))" 2>/dev/null)
    new=$($PY -c "import json;print(json.load(open('$dir/.claude.json')).get('oauthAccount',{}).get('accountUuid',''))" 2>/dev/null)
    if [ -n "$orig" ] && [ -n "$new" ] && [ "$orig" != "$new" ]; then
        printf "${Y}[cl]${N} ⚠ 오염 감지! UUID: $orig → $new\n"
        cp "$bak" "$dir/.claude.json"
        printf "${G}[cl]${N} 복원 완료\n"
    fi
    rm -f "$bak"
}

# ══════════════════════════════════════════════════════════════════════
#  LAUNCH
# ══════════════════════════════════════════════════════════════════════
launch() {
    local name="$1"; shift
    bootstrap

    # find config dir
    local dir
    dir=$($PY -c "
import json,os
d=json.load(open(os.path.expanduser('~/.airgenome/accounts.json')))
for a in d['accounts']:
    if a['name']=='$name': print(a['dir']); break
" 2>/dev/null)

    [ -z "$dir" ] && { printf "${R}[cl]${N} 계정 없음: $name\n"; return 1; }

    # usage
    local usage
    usage=$($PY -c "
import json,os
try:
    u=json.load(open(os.path.expanduser('~/.ccmon-usage-cache.json'))).get('$name',{})
    print(f\"{u.get('session_pct','?')}|{u.get('week_all_pct','?')}\")
except: print('?|?')
" 2>/dev/null)
    local ses="${usage%%|*}" wk="${usage##*|}"

    printf "\n  ${G}>>>${N} ${W}$name${N}  ${D}session=${ses}%%  week=${wk}%%${N}\n"
    printf "  ${D}    $dir${N}\n\n"

    # 오염 백업
    guard_backup "$dir"

    # active 기록
    printf '{"active":"%s"}\n' "$name" > "$STATE"

    # 실행
    local log="/tmp/cl-$$.log"
    CLAUDE_CONFIG_DIR="$dir" script -q "$log" "$CLAUDE"
    local rc=$?

    # 오염 검증
    guard_check "$dir"

    # rate limit 체크 → 자동 전환
    if grep -qiE 'rate.limit|limit.reached|usage.cap|rate_limit_error|too.many.req' "$log" 2>/dev/null; then
        printf "${Y}[cl]${N} ⚡ Rate limit: $name\n"
        local next
        next=$(pick_best)
        local next_name="${next%%|*}"
        if [ -n "$next_name" ] && [ "$next_name" != "$name" ]; then
            printf "${G}[cl]${N} → $next_name 전환\n"
            rm -f "$log"
            launch "$next_name" "$@"
            return $?
        fi
        printf "${R}[cl]${N} 가용 계정 없음\n"
    fi

    rm -f "$log"
    return $rc
}

# ══════════════════════════════════════════════════════════════════════
#  ADD / REMOVE
# ══════════════════════════════════════════════════════════════════════
add_account() {
    local n=$($PY -c "import json,os;print(len(json.load(open(os.path.expanduser('~/.airgenome/accounts.json')))['accounts'])+1)" 2>/dev/null)
    local name="claude$n"
    local dir="$HOME/.claude-$name"
    mkdir -p "$dir"
    printf "${C}[cl]${N} 새 계정 로그인: $name\n"
    CLAUDE_CONFIG_DIR="$dir" "$CLAUDE" login
    if [ -f "$dir/.claude.json" ]; then
        $PY -c "
import json,os
p=os.path.expanduser('~/.airgenome/accounts.json')
d=json.load(open(p))
d['accounts'].append({'name':'$name','dir':'$dir','removed':False})
json.dump(d,open(p,'w'),indent=2)
" 2>/dev/null
        printf "${G}[cl]${N} ✅ $name 추가 완료\n"
    else
        printf "${R}[cl]${N} 로그인 취소\n"
        rmdir "$dir" 2>/dev/null
    fi
}

remove_account() {
    local name="$1"
    $PY -c "
import json,os
p=os.path.expanduser('~/.airgenome/accounts.json')
d=json.load(open(p))
for a in d['accounts']:
    if a['name']=='$name': a['removed']=True
json.dump(d,open(p,'w'),indent=2)
" 2>/dev/null
    printf "${G}[cl]${N} $name 폐기 처리\n"
}

# ══════════════════════════════════════════════════════════════════════
#  CLI ROUTER
# ══════════════════════════════════════════════════════════════════════
case "${1:-}" in
    status|-s)  show_status ;;
    pick|-p)    shift; launch "${1:?usage: cl pick <name>}" ;;
    add|-a)     add_account ;;
    remove|-r)  shift; remove_account "${1:?usage: cl remove <name>}" ;;
    help|-h)
        printf "\n  ${W}cl${N} — Claude Code hexagon gate launcher (airgenome)\n\n"
        printf "  ${C}cl${N}              최적 계정 자동 선택 + 실행\n"
        printf "  ${C}cl status${N}       6축 hexagon gate 현황\n"
        printf "  ${C}cl pick <N>${N}     계정 지정 실행\n"
        printf "  ${C}cl add${N}          새 계정 로그인 추가\n"
        printf "  ${C}cl remove <N>${N}   계정 폐기\n"
        printf "  ${C}cl help${N}         도움말\n\n"
        printf "  ${D}Rate limit → 자동 전환 | 오염 방지 내장${N}\n"
        printf "  ${D}폐기: 이 파일 삭제 + ~/.zshrc 한 줄 제거${N}\n\n"
        ;;
    *)
        bootstrap
        best=$(pick_best)
        name="${best%%|*}"
        [ -n "$name" ] && launch "$name" "$@" || { printf "${R}[cl]${N} 가용 계정 없음\n"; show_status; }
        ;;
esac
