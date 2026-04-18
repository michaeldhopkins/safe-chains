# Configuration

## Claude Code

Run `safe-chains --setup` to automatically configure the hook in `~/.claude/settings.json`. Or manually add:

```json
"hooks": {
  "PreToolUse": [
    {
      "matcher": "Bash",
      "hooks": [
        {
          "type": "command",
          "command": "safe-chains"
        }
      ]
    }
  ]
}
```

Restart your Claude Code sessions to activate the hook. Updating the `safe-chains` binary takes effect immediately.

## OpenCode (experimental)

Generate OpenCode `permission.bash` rules from safe-chains' command list:

```bash
safe-chains --opencode-config > opencode.json
```
