#!/bin/zsh
# iTerm2 프로파일 커맨드에서도 동작하도록 login 환경 수동 로드
[ -f ~/.zprofile ] && source ~/.zprofile
[ -f ~/.zshrc ] && source ~/.zshrc

ORIG_DIR="$(pwd)"
cd ~/Dev/airgenome
OUTPUT=$(~/Dev/hexa-lang/hexa run modules/cl.hexa "$@" 2>&1)
LAUNCH_DIR=$(echo "$OUTPUT" | grep '^LAUNCH:' | sed 's/^LAUNCH://')
cd "$ORIG_DIR"
# Print everything except the LAUNCH marker
echo "$OUTPUT" | grep -v '^LAUNCH:'
# If LAUNCH marker found, exec claude in caller's directory
if [ -n "$LAUNCH_DIR" ]; then
    export CLAUDE_CONFIG_DIR="$LAUNCH_DIR"
    exec ~/.local/bin/claude
else
    echo ""
    echo "[cl] claude 실행 실패 — LAUNCH 마커 없음"
    echo "[cl] 아무 키나 누르면 종료..."
    read -r _
fi
