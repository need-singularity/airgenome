#!/usr/bin/env bash
# bin/build_menubar.sh — T4 menubar build (hexa-only via hexa_v2 transpiler)
# 산출: build/artifacts/airgenome-menubar (static, dlopen objc framework)
set -euo pipefail

ROOT="${AIRGENOME_ROOT:-$HOME/Dev/airgenome}"
HXV2="$HOME/Dev/hexa-lang/self/native/hexa_v2"
RUNTIME="$HOME/Dev/hexa-lang/self/runtime.c"
SRC="$ROOT/bin/menubar.hexa"
ART="$ROOT/build/artifacts"
OUT_C="$ART/menubar.c"
OUT_BIN="$ART/airgenome-menubar"

[ -x "$HXV2"  ] || { echo "❌ hexa_v2 missing: $HXV2"  >&2; exit 1; }
[ -f "$RUNTIME" ] || { echo "❌ runtime.c missing: $RUNTIME" >&2; exit 1; }
[ -f "$SRC"   ] || { echo "❌ src missing: $SRC"   >&2; exit 1; }

mkdir -p "$ART"
cp -f "$RUNTIME" "$ART/runtime.c"

echo "[1/2] hexa_v2 transpile → C"
"$HXV2" "$SRC" "$OUT_C"

echo "[2/2] clang compile → native binary"
clang -O2 -o "$OUT_BIN" "$OUT_C"

echo "✅ built: $OUT_BIN"
ls -la "$OUT_BIN"
