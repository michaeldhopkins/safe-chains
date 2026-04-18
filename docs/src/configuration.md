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

## Cleaning up approved commands

Once safe-chains is active, most of your existing `Bash(...)` approved commands in `~/.claude/settings.json` and `.claude/settings.local.json` are redundant. safe-chains already handles them with stricter, flag-level validation.

More importantly, broad patterns can weaken your security. A pattern like `Bash(bash *)` will approve `bash -c "rm -rf /"` — Claude Code matches the pattern before safe-chains gets a chance to recursively validate the inner command.

Review your approved commands and remove any that safe-chains covers. A good prompt for this:

```
Find every .claude folder on my system — ~/.claude, any .claude
folders at the top of my projects directory, and .claude folders
inside individual repos. For each settings.json and
settings.local.json, check every Bash(...) pattern against
safe-chains (run `safe-chains "command"` to test). Flag overly
broad patterns like Bash(bash *) or Bash(sh *) that bypass
safe-chains' recursive validation. Present me with a suggested
list of changes for each file before making any edits.
```

Or, to clear out all approved Bash commands from every Claude settings file at once:

```bash
find ~/.claude ~/projects -maxdepth 4 -name 'settings*.json' -path '*/.claude/*' | while read f; do
  jq '
    if .approved_commands then .approved_commands |= map(select(startswith("Bash(") | not)) else . end |
    if .permissions.allow then .permissions.allow |= map(select(startswith("Bash(") | not)) else . end
  ' "$f" > "$f.tmp" && mv "$f.tmp" "$f" && echo "Cleaned $f"
done
```

This removes every `Bash(...)` entry but leaves non-Bash permissions (WebFetch, Edit, etc.) untouched.

## OpenCode (experimental)

Generate OpenCode `permission.bash` rules from safe-chains' command list:

```bash
safe-chains --opencode-config > opencode.json
```
