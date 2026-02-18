#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Building and installing safe-chains..."
cargo install --path "$SCRIPT_DIR"

if [ -f "$HOME/.claude/settings.json" ]; then
    if grep -q "safe-chains" "$HOME/.claude/settings.json"; then
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
        echo '            "command": "safe-chains"'
        echo '          }'
        echo '        ]'
        echo '      }'
        echo '    ]'
        echo '  }'
    fi
else
    echo "No ~/.claude/settings.json found. Create one with the hook config after installing Claude Code."
fi
