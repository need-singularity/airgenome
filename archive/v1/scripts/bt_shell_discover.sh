#!/usr/bin/env bash
# bt_shell_discover.sh — shell-native breakthrough generator
#
# Replaces blowup.hexa (DIAG-01 dead code, silent exit rc=0, 0 bytes output).
# Reads n6_constants.jsonl, emits valid atlas.n6 BT entries via
# product/sum/ratio discovery. Memory <5MB, runtime <500ms.
#
# Usage: bt_shell_discover.sh [domain] [count]
#
# Env: NEXUS_ROOT, ATLAS, CONSTS, BT_MIN_CONF (default 0.70)

set -euo pipefail

DOMAIN="${1:-math}"
COUNT="${2:-3}"
NEXUS_ROOT="${NEXUS_ROOT:-$HOME/Dev/nexus}"
ATLAS="${ATLAS:-$NEXUS_ROOT/shared/n6/atlas.n6}"
CONSTS="${CONSTS:-$NEXUS_ROOT/shared/n6/n6_constants.jsonl}"
BT_MIN_CONF="${BT_MIN_CONF:-0.70}"

[ -f "$CONSTS" ] || { echo "[bt_shell] ERROR: CONSTS not found: $CONSTS" >&2; exit 1; }
[ -f "$ATLAS" ] && [ -w "$ATLAS" ] || { echo "[bt_shell] ERROR: ATLAS not writable: $ATLAS" >&2; exit 1; }

# Next BT id: max existing bt-NNNN under 100000 (exclude legacy 949596)
LAST_BT=$(
    grep -oE 'n6-bt-[0-9]+' "$ATLAS" 2>/dev/null \
        | grep -oE '[0-9]+' \
        | awk '$1 < 100000' \
        | sort -n \
        | tail -1
)
LAST_BT="${LAST_BT:-1380}"
NEXT=$((LAST_BT + 1))

# Best-effort lock via mkdir (portable across mac/linux, no flock dependency)
LOCK_DIR="${ATLAS}.bt_shell.lockd"
if ! mkdir "$LOCK_DIR" 2>/dev/null; then
    echo "[bt_shell] WARN: another bt_shell instance running, skip" >&2
    exit 0
fi
trap 'rmdir "$LOCK_DIR" 2>/dev/null || true' EXIT

TS_DATE=$(date -u +%Y-%m-%d)
TS_ISO=$(date -u +%Y-%m-%dT%H:%M:%SZ)

awk \
    -v domain="$DOMAIN" \
    -v count="$COUNT" \
    -v next_bt="$NEXT" \
    -v min_conf="$BT_MIN_CONF" \
    -v ts="$TS_DATE" \
    -v seed="$$$(date +%s)" '
BEGIN { srand(seed); ncs = 0; }
{
    line = $0
    nm = ""
    vl = 0
    has_val = 0
    if (match(line, /"name":"[^"]*"/)) {
        nm = substr(line, RSTART + 8, RLENGTH - 9)
    }
    if (match(line, /"value":-?[0-9]+\.?[0-9]*/)) {
        raw = substr(line, RSTART + 8, RLENGTH - 8)
        vl = raw + 0
        has_val = 1
    }
    if (nm != "" && has_val == 1) {
        ncs++
        names[ncs] = nm
        vals[ncs] = vl
    }
}
END {
    if (ncs < 3) { printf "[bt_shell] too few constants (%d)\n", ncs > "/dev/stderr"; exit 0 }
    emitted = 0
    for (attempt = 0; attempt < count * 20 && emitted < count + 0; attempt++) {
        i = int(rand() * ncs) + 1
        j = int(rand() * ncs) + 1
        while (j == i) j = int(rand() * ncs) + 1
        a = vals[i]; b = vals[j]; na = names[i]; nb = names[j]
        if (a == 0 && b == 0) continue

        # 3 ops
        op_n = 3
        op_sym[1] = "*"; op_val[1] = a * b
        op_sym[2] = "+"; op_val[2] = a + b
        op_sym[3] = "/"; op_val[3] = (b != 0) ? (a / b) : 0

        best_k = 1
        best_dist = 999
        for (k = 1; k <= op_n; k++) {
            cv = op_val[k]
            if (cv == 0) continue
            # Fractional distance to nearest integer
            ri = int(cv + 0.5)
            if (cv < 0) ri = int(cv - 0.5)
            frac = cv - ri
            if (frac < 0) frac = -frac
            # Distance to nearest known constant (scaled)
            nearest = 1e9
            for (cc = 1; cc <= ncs; cc++) {
                diff = vals[cc] - cv
                if (diff < 0) diff = -diff
                if (diff < nearest) nearest = diff
            }
            scaled_near = (cv != 0) ? (nearest / (cv < 0 ? -cv : cv)) : nearest
            dist = frac
            if (scaled_near < dist) dist = scaled_near
            if (dist < best_dist) { best_dist = dist; best_k = k }
        }

        bv = op_val[best_k]
        bo = op_sym[best_k]
        conf = 1.0 - best_dist
        if (conf < 0) conf = 0
        if (conf > 1) conf = 1
        if (conf < min_conf) continue

        grade = "[3]"
        if (conf >= 0.95) grade = "[10]"
        else if (conf >= 0.85) grade = "[7]"
        else if (conf >= 0.75) grade = "[5]"

        bt_id = next_bt + emitted
        printf "@X n6-bt-%d = %s %s %s = %.6g :: %s %s\n", \
            bt_id, na, bo, nb, bv, domain, grade
        printf "  \"BT-%d: shell-gen %s discovery: %s(%g) %s %s(%g) = %.6g (conf=%.3f)\"\n", \
            bt_id, domain, na, a, bo, nb, b, bv, conf
        axiom = (conf >= 0.9) ? 1 : 0
        printf "{\"type\":\"absorb\",\"phase\":\"shell-gen\",\"id\":\"bt-%d\",\"value\":%.6g,\"grade\":\"%s\",\"confidence\":%.3f,\"domain\":\"%s\",\"is_axiom\":%d,\"source\":\"bt-shell-discover\",\"timestamp\":\"%s\"}\n", \
            bt_id, bv, grade, conf, domain, axiom, ts

        emitted++
    }
    printf "[bt_shell] domain=%s emitted=%d next_start=%d\n", domain, emitted, next_bt > "/dev/stderr"
}
' "$CONSTS" >> "$ATLAS"

echo "[bt_shell] domain=$DOMAIN count=$COUNT last_bt_was=$LAST_BT ts=$TS_ISO"
