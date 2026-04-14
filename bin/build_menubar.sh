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

echo "[1/3] hexa_v2 transpile → C"
"$HXV2" "$SRC" "$OUT_C"

echo "[2/3] FFI marshalling post-process (TAG_STR 포인터 + msg_float ABI)"
# hexa_v2 0.x codegen 버그 우회:
#   1) (X.tag==TAG_INT?X.i:(int64_t)X.f) 는 TAG_STR 일 때 포인터 소실
#      → hexa_ffi_marshal_arg(X) 로 교체 (TAG_STR 포인터, FLOAT bit-reinterpret 등 전부 처리)
#   2) msg_float 은 int64_t arg typedef 로 호출 → ARM64 ABI 에서 d0 아닌 x2 에 전달되어 CGFloat 소실
#      → __ffi_ftyp_msg_float 시그니처를 double 로, 호출부도 double 로
perl -i -pe 's/\(([a-zA-Z_]\w*)\.tag==TAG_INT\?\1\.i:\(int64_t\)\1\.f\)/hexa_ffi_marshal_arg($1)/g' "$OUT_C"
# msg_float 특화 — CGFloat ABI 수정
perl -i -pe 's{typedef int64_t \(\*__ffi_ftyp_msg_float\)\(int64_t, int64_t, int64_t\);}{typedef int64_t (*__ffi_ftyp_msg_float)(int64_t, int64_t, double);}' "$OUT_C"
perl -i -pe 's{HexaVal msg_float\(HexaVal obj, HexaVal sel, HexaVal a1\) \{\n    int64_t __r = \(\(__ffi_ftyp_msg_float\)__ffi_sym_msg_float\)\(hexa_ffi_marshal_arg\(obj\), hexa_ffi_marshal_arg\(sel\), hexa_ffi_marshal_arg\(a1\)\);}{HexaVal msg_float(HexaVal obj, HexaVal sel, HexaVal a1) \{\n    double _da1 = (a1.tag==TAG_FLOAT?a1.f:(a1.tag==TAG_INT?(double)a1.i:0.0));\n    int64_t __r = ((__ffi_ftyp_msg_float)__ffi_sym_msg_float)(hexa_ffi_marshal_arg(obj), hexa_ffi_marshal_arg(sel), _da1);}s' "$OUT_C"

echo "[3/3] clang compile → native binary (AppKit + CoreFoundation link)"
clang -O2 -framework AppKit -framework CoreFoundation -o "$OUT_BIN" "$OUT_C"

echo "✅ built: $OUT_BIN"
ls -la "$OUT_BIN"
