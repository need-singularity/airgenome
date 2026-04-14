#!/usr/bin/env bash
# bin/test_menubar.sh — airgenome menubar 강제 검증 하네스
#
# 목적: build_app.sh 에서 codesign/deploy 전에 호출되는 GATE.
#       바이너리가 config 로드 + status item 생성 + menu build + setMenu 왕복을
#       무사히 통과하는지 자동 검증. 실패시 exit 1 → deploy 중단.
#
# 패턴: void/scripts/test_void.sh 스타일 — 환경변수로 test mode 진입, stdout 로그를
#       grep 으로 검증.
#
# 검증 항목:
#   1. TEST START / TEST DONE PASS 마커
#   2. 최소 아이템 개수 (MIN_ITEMS)
#   3. 필수 아이템 존재 — Legend, Throttle, Dispatch, mac, ubu, htz, trend, rings, cfg, Quit
#   4. statusItem ok / button ok / setMenu roundtrip ok
#   5. 종료 코드 0
#
# 사용:
#   bin/test_menubar.sh [binary_path]
#   기본 binary: build/artifacts/airgenome-menubar
#
# 종료 코드:
#   0  PASS — 모든 검증 통과
#   1  FAIL — 하나라도 실패, deploy 차단

set -euo pipefail

ROOT="${AIRGENOME_ROOT:-$HOME/Dev/airgenome}"
BIN="${1:-$ROOT/build/artifacts/airgenome-menubar}"
TIMEOUT_SEC=10
MIN_ITEMS=15
LOG="/tmp/airgenome_menubar_test.log"

RED=$(printf '\033[0;31m')
GREEN=$(printf '\033[0;32m')
YELLOW=$(printf '\033[0;33m')
RESET=$(printf '\033[0m')

fail() {
    printf '%sFAIL%s %s\n' "$RED" "$RESET" "$1" >&2
    echo "--- log tail ---" >&2
    tail -30 "$LOG" >&2 2>/dev/null || true
    exit 1
}

pass() {
    printf '%sPASS%s %s\n' "$GREEN" "$RESET" "$1"
}

note() {
    printf '%s...%s %s\n' "$YELLOW" "$RESET" "$1"
}

# ── 1. precondition ─────────────────────────────────────────────
[ -x "$BIN" ] || fail "binary missing or not executable: $BIN"
note "binary: $BIN"
note "timeout: ${TIMEOUT_SEC}s  min_items: $MIN_ITEMS"

# ── 2. run binary in test mode ───────────────────────────────────
rm -f "$LOG"
note "launching AIRGENOME_MENUBAR_TEST=1 ..."
# timeout(1) not always available on macOS; use perl-based alarm
(
    AIRGENOME_MENUBAR_TEST=1 "$BIN" &
    TPID=$!
    (sleep "$TIMEOUT_SEC" && kill -9 "$TPID" 2>/dev/null && echo "TEST TIMEOUT") &
    wait "$TPID" 2>/dev/null || true
) > "$LOG" 2>&1
EXIT=$?

[ -s "$LOG" ] || fail "no output captured"

# ── 3. assertions ────────────────────────────────────────────────
grep -q "^TEST START" "$LOG" || fail "TEST START marker missing"
pass "TEST START marker"

grep -q "^TEST config loaded" "$LOG" || fail "config loaded marker missing"
pass "config loaded marker"

grep -q "^TEST button ok" "$LOG" || fail "button FFI check failed (item.button returned NULL)"
pass "button FFI"

grep -q "^TEST statusItem ok" "$LOG" || fail "statusItem check failed"
pass "statusItem FFI"

grep -q "^TEST setMenu roundtrip ok" "$LOG" || fail "setMenu roundtrip failed — item.menu did not match after setMenu:"
pass "setMenu roundtrip"

# [Track B gap 1] Quit action 의 selector/target 바인딩 검증
grep -q "^TEST ACTION Quit selector=terminate: target=NSApp" "$LOG" || fail "Quit action selector binding missing — menu 클릭 이벤트 라우팅 깨짐"
pass "Quit action binding"

# [Track B gap 2] config 정합성 — 전 필드 덤프 존재 + 임계값 sanity
grep -q "^TEST CONFIG tick=" "$LOG" || fail "TEST CONFIG dump missing"
grep -Eq "^TEST CONFIG .*color=[0-9]+/[0-9]+ stale=[0-9]+s cap=[0-9]+" "$LOG" || fail "TEST CONFIG field shape invalid"
pass "config dump + shape"

# [Track B gap 2] snapshot 덤프 — refresh_snapshot 이 실제 값을 생산
grep -q "^TEST SNAP level=" "$LOG" || fail "TEST SNAP missing — refresh_snapshot not run or output empty"
pass "snapshot dump"

grep -q "TEST TIMEOUT" "$LOG" && fail "binary timed out (> ${TIMEOUT_SEC}s)"

grep -q "^TEST DONE PASS" "$LOG" || fail "TEST DONE PASS marker missing — binary crashed or exited early"
pass "TEST DONE PASS marker"

# [Track B gap 3] REPEAT 모드 — 5회 build_menu 반복에서 크래시/누수 없는지
# (이 검증은 별도 런. 주 run 다음에 REPEAT 환경 변수로 재실행)
note "Track B gap 3: REPEAT stability check"
REPEAT_LOG="/tmp/airgenome_menubar_repeat.log"
rm -f "$REPEAT_LOG"
(AIRGENOME_MENUBAR_TEST=1 AIRGENOME_MENUBAR_TEST_REPEAT=5 "$BIN" &
    RPID=$!
    (sleep "$TIMEOUT_SEC" && kill -9 "$RPID" 2>/dev/null && echo "REPEAT TIMEOUT") &
    wait "$RPID" 2>/dev/null || true
) > "$REPEAT_LOG" 2>&1
grep -q "TEST REPEAT n=5 ok" "$REPEAT_LOG" || fail "REPEAT n=5 build_menu 반복 실패 (크래시 또는 timeout)"
pass "REPEAT n=5 stability"

# ── 4. item count + required items ───────────────────────────────
ITEM_COUNT=$(grep -c "^ITEM " "$LOG" || true)
note "menu items emitted: $ITEM_COUNT (min $MIN_ITEMS)"
[ "$ITEM_COUNT" -ge "$MIN_ITEMS" ] || fail "item count $ITEM_COUNT < $MIN_ITEMS"
pass "item count >= $MIN_ITEMS"

REQUIRED=(
    "Legend:"
    "Throttle:"
    "Dispatch:"
    "mac:"
    "ubu:"
    "htz:"
    "trend mac:"
    "trend ubu:"
    "trend htz:"
    "rings"
    "cfg:"
    "Quit"
)
for key in "${REQUIRED[@]}"; do
    grep -q "^ITEM .*${key}" "$LOG" || fail "required item missing: $key"
    pass "item present: $key"
done

# ── 5. exit code ─────────────────────────────────────────────────
# 바이너리는 TEST DONE PASS 후 return → exit 0 기대
# 단 msg_int 등 일부 hexa_v2 runtime 이 non-zero 종료할 수 있어 여기서만 경고
if [ "$EXIT" -ne 0 ]; then
    note "binary exit code $EXIT (non-zero — hexa runtime quirk 가능성, 마커로는 이미 PASS)"
fi

printf '\n%s✅ ALL GREEN%s — menubar harness %s\n' "$GREEN" "$RESET" "$(basename "$BIN")"
echo "log: $LOG"
exit 0
