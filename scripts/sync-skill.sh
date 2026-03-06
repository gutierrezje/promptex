#!/usr/bin/env bash
# Sync the prompt-history skill source into the global agent install.
# Run this after editing anything under skills/prompt-history/.
#
# First-time setup: npx skills add gutierrezje/promptex -g

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$REPO_ROOT/skills/prompt-history"
DEST="$HOME/.agents/skills/prompt-history"

if [ ! -d "$DEST" ]; then
  echo "error: global skill install not found at $DEST" >&2
  echo "Run 'npx skills add gutierrezje/promptex -g' first." >&2
  exit 1
fi

cp -r "$SRC/" "$DEST/"
echo "synced skills/prompt-history → $DEST"
