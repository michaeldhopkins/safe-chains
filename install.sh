#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
HOOK_DIR="$HOME/.claude/hooks"
HOOK_PATH="$HOOK_DIR/safe-chains.sh"

mkdir -p "$HOOK_DIR"

if [ -f "$HOOK_PATH" ] && [ ! -L "$HOOK_PATH" ]; then
    echo "Backing up existing $HOOK_PATH to $HOOK_PATH.bak"
    mv "$HOOK_PATH" "$HOOK_PATH.bak"
fi

ln -sf "$SCRIPT_DIR/safe-chains.sh" "$HOOK_PATH"
echo "Symlinked $HOOK_PATH -> $SCRIPT_DIR/safe-chains.sh"

if [ -f "$HOME/.claude/settings.json" ]; then
    if grep -q "safe-chains.sh" "$HOME/.claude/settings.json"; then
        echo "Hook config found in settings.json."
    else
        echo ""
        echo "Add this to your ~/.claude/settings.json hooks section:"
        echo ""
        echo '  "hooks": {'
        echo '    "PreToolUse": ['
        echo '      {'
        echo '        "matcher": "Bash",'
        echo '        "hooks": ['
        echo '          {'
        echo '            "type": "command",'
        echo "            \"command\": \"$HOOK_PATH\""
        echo '          }'
        echo '        ]'
        echo '      }'
        echo '    ]'
        echo '  }'
    fi
else
    echo "No ~/.claude/settings.json found. Create one with the hook config after installing Claude Code."
fi
