#!/bin/bash
# Block CREATION of .py, .rs, .sh files (HEXA-FIRST rule)
# Existing file edits are allowed (Edit tool), only new file creation blocked (Write tool)

INPUT=$(cat)
TOOL=$(echo "$INPUT" | jq -r '.tool_name // empty')
FILE=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

if [[ -z "$FILE" ]]; then
  exit 0
fi

# Only block Write (new file creation), not Edit (existing file modification)
if [[ "$TOOL" == "Write" && "$FILE" =~ \.(py|rs|sh)$ ]]; then
  # Allow if file already exists (overwrite)
  if [[ -f "$FILE" ]]; then
    exit 0
  fi
  echo "BLOCKED: $FILE — .py/.rs/.sh 신규 작성 금지. .hexa로 작성하세요. (CLAUDE.md 규칙 0)" >&2
  exit 2
fi

exit 0
