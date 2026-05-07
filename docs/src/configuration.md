# Configuration

safe-chains integrates with multiple agentic CLI coding tools. List the supported targets with:

```bash
safe-chains --list-tools
```

Install for a specific tool:

```bash
safe-chains --setup                   # default: Claude Code
safe-chains --setup --tool=codex      # Codex (OpenAI)
safe-chains --setup --auto-detect     # install for every detected tool
```

## Claude Code

Run `safe-chains --setup` (or `--setup --tool=claude`) to automatically configure the hook in `~/.claude/settings.json`. Or manually add:

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

## Codex (OpenAI)

Run `safe-chains --setup --tool=codex` to write `~/.codex/hooks.json` with safe-chains as a `PreToolUse` hook. Or manually add to `~/.codex/hooks.json`:

```json
{
  "PreToolUse": [
    {
      "matcher": "Bash",
      "hooks": [
        {
          "type": "command",
          "command": "safe-chains hook codex"
        }
      ]
    }
  ]
}
```

Codex requires `[features] codex_hooks = true` in `~/.codex/config.toml` for hooks to fire. Add it manually if it isn't already there:

```toml
[features]
codex_hooks = true
```

Restart your Codex sessions after the first install. Updating the `safe-chains` binary takes effect immediately.

## Cursor CLI

Cursor exposes a dedicated `beforeShellExecution` event that fires only on shell calls — cleaner than a generic pre-tool hook. Run `safe-chains --setup --tool=cursor` to install. The config goes to `~/.cursor/hooks.json`:

```json
{
  "version": 1,
  "hooks": {
    "beforeShellExecution": [
      {
        "command": "safe-chains hook cursor",
        "timeout": 30
      }
    ]
  }
}
```

Cursor hooks fail-open by default. If you want safe-chains failures to block (rather than silently letting commands through), add `"failClosed": true` to the entry.

## Gemini CLI

Run `safe-chains --setup --tool=gemini` to write `~/.gemini/settings.json`. Gemini's hook event is `BeforeTool` (PascalCase) and the response key is `decision` (`allow` / `deny` — there's no `ask`). Manual config:

```json
{
  "hooks": {
    "BeforeTool": [
      {
        "matcher": "^run_shell_command$",
        "hooks": [
          {
            "type": "command",
            "command": "safe-chains hook gemini",
            "timeout": 60000
          }
        ]
      }
    ]
  }
}
```

Note Gemini's `timeout` is in milliseconds (other vendors use seconds).

## Qwen Code

Run `safe-chains --setup --tool=qwen` to write `~/.qwen/settings.json`. Qwen mirrors Claude Code's hook envelope verbatim. Manual config:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "^Bash$",
        "hooks": [
          {
            "type": "command",
            "command": "safe-chains hook qwen",
            "timeout": 60000
          }
        ]
      }
    ]
  }
}
```

## Factory Droid

Run `safe-chains --setup --tool=droid` to write `~/.factory/settings.json`. Droid's bash tool is named `Execute` (not `Bash`), and Droid requires absolute paths for hook commands — the installer resolves the safe-chains binary's absolute path at install time. Manual config (substitute the absolute path of your `safe-chains` binary):

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Execute",
        "hooks": [
          {
            "type": "command",
            "command": "/usr/local/bin/safe-chains hook droid",
            "timeout": 60
          }
        ]
      }
    ]
  }
}
```

## GitHub Copilot CLI

Copilot's hook config lives in `.github/hooks/*.json` (per-repo) or `~/.github/hooks/*.json` (user-global, files merge). Run `safe-chains --setup --tool=copilot` to write `~/.github/hooks/safe-chains.json`. Copilot's quirks: the response is a flat object (no `hookSpecificOutput` wrapper), the script-path field is `bash` (not `command`), and `toolArgs` is a JSON-encoded *string* on stdin (the safe-chains adapter double-decodes it). Manual config (substitute absolute path):

```json
{
  "version": 1,
  "hooks": {
    "preToolUse": [
      {
        "type": "command",
        "bash": "/usr/local/bin/safe-chains hook copilot",
        "comment": "safe-chains: validate every Bash tool call before it runs.",
        "timeoutSec": 60
      }
    ]
  }
}
```

As of late 2025 only `permissionDecision: "deny"` is honored by Copilot's permission system; safe-chains emits `"allow"` envelopes anyway so future Copilot releases that honor the full schema get the upgrade for free.

## OpenCode (experimental)

Generate OpenCode `permission.bash` rules from safe-chains' command list:

```bash
safe-chains --opencode-config > opencode.json
```
