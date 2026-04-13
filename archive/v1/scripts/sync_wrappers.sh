#!/bin/bash
# scripts/sync_wrappers.sh — sync gate/wrappers/ -> ~/.hx/bin/
# Source of truth: gate/wrappers/ (repo-tracked, patched wrappers)
# Target: ~/.hx/bin/ (runtime PATH shims)
#
# Idempotent: only copies when files differ.
# Usage: bash scripts/sync_wrappers.sh [-v|--verbose] [-n|--dry-run]

set -euo pipefail

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SRC_DIR="$REPO_DIR/gate/wrappers"
DST_DIR="$HOME/.hx/bin"

# ── Wrappers to sync (gate scripts that survived cargo build) ──
# Excludes: _budget.hexa (hexa source, not a shell wrapper)
#           patched/ (archive of older versions)
WRAPPERS=(
  _cache.sh
  cargo
  hexa
  hexa-bin
  hexa-launcher
  python3
  rustc
  sh-run
)

VERBOSE=0
DRY_RUN=0

for arg in "$@"; do
  case "$arg" in
    -v|--verbose) VERBOSE=1 ;;
    -n|--dry-run) DRY_RUN=1; VERBOSE=1 ;;
    -h|--help)
      echo "Usage: $0 [-v|--verbose] [-n|--dry-run]"
      echo "  Syncs gate/wrappers/ -> ~/.hx/bin/ (only when files differ)"
      exit 0
      ;;
  esac
done

_log() { [ "$VERBOSE" = "1" ] && echo "$@"; }

# ── Pre-flight ──
if [ ! -d "$SRC_DIR" ]; then
  echo "FATAL: source directory not found: $SRC_DIR" >&2
  exit 1
fi

if [ ! -d "$DST_DIR" ]; then
  echo "Creating target directory: $DST_DIR"
  [ "$DRY_RUN" = "0" ] && mkdir -p "$DST_DIR"
fi

# ── Sync loop ──
copied=0
skipped=0
missing_src=0

for w in "${WRAPPERS[@]}"; do
  src="$SRC_DIR/$w"
  dst="$DST_DIR/$w"

  if [ ! -f "$src" ]; then
    echo "WARN: source missing: $src" >&2
    missing_src=$((missing_src + 1))
    continue
  fi

  # If destination is a symlink, remove it first (replace with real file)
  if [ -L "$dst" ]; then
    _log "  unlink symlink: $dst -> $(readlink "$dst")"
    [ "$DRY_RUN" = "0" ] && rm "$dst"
  fi

  if [ -f "$dst" ] && diff -q "$src" "$dst" >/dev/null 2>&1; then
    _log "  skip (identical): $w"
    skipped=$((skipped + 1))
  else
    if [ -f "$dst" ]; then
      _log "  update: $w"
    else
      _log "  install: $w"
    fi
    if [ "$DRY_RUN" = "0" ]; then
      cp "$src" "$dst"
      chmod +x "$dst"
    fi
    copied=$((copied + 1))
  fi
done

# ── Summary ──
echo "[sync_wrappers] copied=$copied skipped=$skipped missing=$missing_src src=$SRC_DIR dst=$DST_DIR"
[ "$DRY_RUN" = "1" ] && echo "(dry-run: no files were modified)"

exit 0
