#!/usr/bin/env bash
# bin/improve_review.sh — Track H Step 2: patch 안전성 판정기 (stateless)
#
# 목적: self-improvement pipeline 의 2단계에서 claude -p 가 생성한 unified
#       diff 가 main tree 에 적용해도 안전한지 판정. 이 스크립트는 오직 판정만,
#       실제 적용/커밋은 절대 하지 않음 (단독 테스트 가능).
#
# 입력: 하나의 patch 파일 경로 (unified diff 형식, `git diff` 또는
#       `git diff --no-prefix` 스타일)
# 출력: stdout 에 판정 JSON 한 줄 + 상세 로그
# exit code:
#   0 = SAFE     — 모든 gate 통과, apply 해도 됨
#   1 = BLOCKED  — blacklist 또는 규칙 위반, apply 절대 금지
#   2 = AMBIGUOUS — 판단 불확실 (수동 검토 필요)
#
# 규칙 (CLAUDE.md 와 일치):
#  - 한 파일만 수정 (multi-file diff = BLOCKED)
#  - diff 크기 cap: 추가+삭제 합계 ±30 라인
#  - 경로 blacklist: core/**, archive/**, shared/config/roadmap/**, .github/**,
#                    *.plist, *.ring, forge/**, CLAUDE.md, run.hexa, .git/**
#  - 키워드 blacklist: rm -, unlink, git reset --hard, git push --force,
#                       DROP TABLE, truncate, SIGKILL, bootout, --force, --yes
#  - 파일 생성/삭제 금지 (modify only)
#  - git apply --check 실패 = BLOCKED
#
# 사용:
#   bin/improve_review.sh /path/to/patch.diff
#   bin/improve_review.sh --self-test   # 내장 테스트 케이스 실행

set -euo pipefail

ROOT="${AIRGENOME_ROOT:-$HOME/Dev/airgenome}"
MAX_LINES=30

RED=$(printf '\033[0;31m')
GREEN=$(printf '\033[0;32m')
YELLOW=$(printf '\033[0;33m')
RESET=$(printf '\033[0m')

blocked() {
    printf '%sBLOCKED%s %s\n' "$RED" "$RESET" "$1" >&2
    printf '{"verdict":"blocked","reason":"%s"}\n' "$1"
    exit 1
}

ambiguous() {
    printf '%sAMBIGUOUS%s %s\n' "$YELLOW" "$RESET" "$1" >&2
    printf '{"verdict":"ambiguous","reason":"%s"}\n' "$1"
    exit 2
}

ok() {
    printf '%sOK%s %s\n' "$GREEN" "$RESET" "$1"
}

# ── 경로 blacklist (glob 패턴, bash [[ == ]] 매칭) ────────────────
PATH_BLACKLIST=(
    "core/*"
    "archive/*"
    "shared/config/roadmap/*"
    ".github/*"
    "*.plist"
    "*.ring"
    "forge/*"
    "CLAUDE.md"
    "run.hexa"
    ".git/*"
    "*/LaunchAgents/*"
    "shared/launchagents/*"
)

# ── 키워드 blacklist ────────────────────────────────────────────
KEYWORD_BLACKLIST=(
    "rm -rf"
    "rm -r"
    "unlink("
    "git reset --hard"
    "git push --force"
    "DROP TABLE"
    "truncate"
    "SIGKILL"
    "bootout"
    " --force"
    " --yes"
    "eval \""
    "exec(rm"
    "exec(\"rm"
)

validate_patch() {
    local patch="$1"
    [ -f "$patch" ] || blocked "patch file not found: $patch"
    [ -s "$patch" ] || blocked "patch file empty"

    # 1. 멀티파일 검사 — diff --git 헤더 개수
    local file_count
    file_count=$(grep -c '^diff --git ' "$patch" || true)
    if [ "$file_count" -eq 0 ]; then
        blocked "no 'diff --git' header — not a git diff"
    fi
    if [ "$file_count" -gt 1 ]; then
        blocked "multi-file diff — $file_count files changed, expected 1"
    fi
    ok "single-file diff (1 of 1)"

    # 2. 파일 경로 추출 — diff --git a/<path> b/<path> → <path>
    local target_path
    target_path=$(grep -m1 '^diff --git ' "$patch" | sed -E 's|^diff --git a/([^ ]+) b/.*|\1|')
    [ -z "$target_path" ] && blocked "could not parse target path"
    ok "target: $target_path"

    # 3. 파일 생성/삭제 검사
    if grep -q '^new file mode' "$patch"; then
        blocked "creates new file — modify only"
    fi
    if grep -q '^deleted file mode' "$patch"; then
        blocked "deletes file — modify only"
    fi
    ok "modify-only (no create/delete)"

    # 4. 경로 blacklist
    for pat in "${PATH_BLACKLIST[@]}"; do
        case "$target_path" in
            $pat) blocked "path matches blacklist: $pat ($target_path)" ;;
        esac
    done
    ok "path not in blacklist"

    # 5. diff 크기 cap (추가 + 삭제 라인 수, 헤더 제외)
    local added deleted total
    added=$(grep -c '^\+[^+]' "$patch" || true)
    deleted=$(grep -c '^-[^-]' "$patch" || true)
    total=$((added + deleted))
    if [ "$total" -gt "$MAX_LINES" ]; then
        blocked "diff too large: +$added -$deleted = $total lines (cap $MAX_LINES)"
    fi
    ok "diff size: +$added -$deleted = $total lines (cap $MAX_LINES)"

    # 6. 키워드 blacklist — 추가된 라인에만 검사 (삭제는 위험 없음)
    local kw found=""
    for kw in "${KEYWORD_BLACKLIST[@]}"; do
        if grep -q "^+.*${kw}" "$patch"; then
            found="$kw"
            break
        fi
    done
    [ -n "$found" ] && blocked "blacklisted keyword in added lines: '$found'"
    ok "no blacklisted keywords"

    # 7. git apply --check (없으면 ambiguous)
    if command -v git >/dev/null 2>&1; then
        if (cd "$ROOT" && git apply --check "$patch" 2>/dev/null); then
            ok "git apply --check passes"
        else
            blocked "git apply --check failed (patch不 apply)"
        fi
    else
        ambiguous "git not available for --check"
    fi

    # 판정
    printf '\n%s✅ SAFE%s — %s (%s lines)\n' "$GREEN" "$RESET" "$target_path" "$total"
    printf '{"verdict":"safe","file":"%s","added":%d,"deleted":%d,"total":%d}\n' \
        "$target_path" "$added" "$deleted" "$total"
    exit 0
}

# ── 내장 self-test ──────────────────────────────────────────────
self_test() {
    local tmpdir; tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' EXIT

    local passed=0 total=0

    assert_verdict() {
        local expected="$1" desc="$2" patch="$3"
        total=$((total+1))
        local actual_exit=0
        bash "$0" "$patch" >/dev/null 2>&1 || actual_exit=$?
        local actual="unknown"
        case "$actual_exit" in
            0) actual="safe" ;;
            1) actual="blocked" ;;
            2) actual="ambiguous" ;;
        esac
        if [ "$actual" = "$expected" ]; then
            passed=$((passed+1))
            printf '  %s✓%s %-45s → %s\n' "$GREEN" "$RESET" "$desc" "$actual"
        else
            printf '  %s✗%s %-45s → expected %s, got %s\n' "$RED" "$RESET" "$desc" "$expected" "$actual"
        fi
    }

    # Case 1: 정상 단일 파일 소규모 수정 → safe
    cat > "$tmpdir/ok.diff" <<'EOF'
diff --git a/bin/menubar.hexa b/bin/menubar.hexa
index 1111111..2222222 100644
--- a/bin/menubar.hexa
+++ b/bin/menubar.hexa
@@ -1,3 +1,4 @@
 // header
+// new comment
 let x = 1
 let y = 2
EOF
    assert_verdict blocked "정상 단일파일 수정 (git apply 실패 예상)" "$tmpdir/ok.diff"
    # NOTE: git apply --check 는 real index 없으면 실패 → blocked 예상 (tmp diff 는 index hash mock)

    # Case 2: 멀티파일 → blocked
    cat > "$tmpdir/multi.diff" <<'EOF'
diff --git a/bin/a.sh b/bin/a.sh
index 111..222 100644
--- a/bin/a.sh
+++ b/bin/a.sh
@@ -1 +1,2 @@
 line
+new
diff --git a/bin/b.sh b/bin/b.sh
index 333..444 100644
--- a/bin/b.sh
+++ b/bin/b.sh
@@ -1 +1,2 @@
 line
+new
EOF
    assert_verdict blocked "멀티파일 diff" "$tmpdir/multi.diff"

    # Case 3: 경로 blacklist (core/*) → blocked
    cat > "$tmpdir/core.diff" <<'EOF'
diff --git a/core/core.hexa b/core/core.hexa
index 111..222 100644
--- a/core/core.hexa
+++ b/core/core.hexa
@@ -1 +1,2 @@
 line
+injected
EOF
    assert_verdict blocked "core/** 경로 (blacklist)" "$tmpdir/core.diff"

    # Case 4: 파일 생성 → blocked
    cat > "$tmpdir/newfile.diff" <<'EOF'
diff --git a/bin/newfile.sh b/bin/newfile.sh
new file mode 100644
index 000..222
--- /dev/null
+++ b/bin/newfile.sh
@@ -0,0 +1 @@
+echo hello
EOF
    assert_verdict blocked "새 파일 생성" "$tmpdir/newfile.diff"

    # Case 5: 키워드 rm -rf → blocked
    cat > "$tmpdir/rmrf.diff" <<'EOF'
diff --git a/bin/foo.sh b/bin/foo.sh
index 111..222 100644
--- a/bin/foo.sh
+++ b/bin/foo.sh
@@ -1 +1,2 @@
 line
+rm -rf /tmp/x
EOF
    assert_verdict blocked "rm -rf 키워드" "$tmpdir/rmrf.diff"

    # Case 6: diff 크기 초과 → blocked
    {
        echo "diff --git a/bin/big.sh b/bin/big.sh"
        echo "index 111..222 100644"
        echo "--- a/bin/big.sh"
        echo "+++ b/bin/big.sh"
        echo "@@ -1 +1,50 @@"
        echo " line"
        for i in $(seq 1 50); do echo "+new line $i"; done
    } > "$tmpdir/big.diff"
    assert_verdict blocked "30 lines 초과" "$tmpdir/big.diff"

    # Case 7: 파일 삭제 → blocked
    cat > "$tmpdir/delete.diff" <<'EOF'
diff --git a/bin/old.sh b/bin/old.sh
deleted file mode 100755
index 111..000
--- a/bin/old.sh
+++ /dev/null
@@ -1 +0,0 @@
-echo hello
EOF
    assert_verdict blocked "파일 삭제" "$tmpdir/delete.diff"

    # Case 8: CLAUDE.md 변경 → blocked
    cat > "$tmpdir/claudemd.diff" <<'EOF'
diff --git a/CLAUDE.md b/CLAUDE.md
index 111..222 100644
--- a/CLAUDE.md
+++ b/CLAUDE.md
@@ -1 +1,2 @@
 # project
+new rule
EOF
    assert_verdict blocked "CLAUDE.md 직접 편집" "$tmpdir/claudemd.diff"

    echo
    printf '%s─── self-test: %d/%d passed ───%s\n' \
        "$([ $passed -eq $total ] && echo "$GREEN" || echo "$RED")" \
        "$passed" "$total" "$RESET"
    [ "$passed" -eq "$total" ] && exit 0 || exit 1
}

# ── main ───────────────────────────────────────────────────────
case "${1:-}" in
    --self-test|self-test)
        self_test
        ;;
    "")
        echo "usage: $0 <patch_file>" >&2
        echo "       $0 --self-test" >&2
        exit 2
        ;;
    *)
        validate_patch "$1"
        ;;
esac
