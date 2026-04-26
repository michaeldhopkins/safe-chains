# `safe-chains`

A command safety checker that auto-allows safe bash commands without prompting. Works as a Claude Code pre-hook, a CLI tool, or an OpenCode plugin.

safe-chains knows 450+ commands. For each one it validates specific subcommands and flags — allowing `git log` but not `git push`, allowing `sed 's/foo/bar/'` but not `sed -i`. Commands in piped chains, `&&`, and `;` sequences are each validated independently. Compound commands (`for`, `while`, `if`) are parsed recursively.

[Documentation and supported command reference](https://www.michaeldhopkins.com/docs/safe-chains/)

## Install

```bash
brew install michaeldhopkins/tap/safe-chains
```

Or `cargo install safe-chains`, or download a binary from [GitHub Releases](https://github.com/michaeldhopkins/safe-chains/releases/latest).

## Configure

```bash
safe-chains --setup
```

This adds the Claude Code pre-hook to `~/.claude/settings.json`. Restart Claude Code to activate. See the [documentation](https://www.michaeldhopkins.com/docs/safe-chains/configuration.html) for manual setup and OpenCode configuration.

### OpenAI codex

Manually add this to `~/.claude/config.toml`:

```toml
[features]
codex_hooks = true
```

And add this to `~/.claude/hooks.json`:

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

Restart your codex sessions to activate the hook. Once configured, updating the `safe-chains` binary takes effect immediately.

Requires `safe-chains` in PATH.

## Usage

With the hook configured, safe-chains runs automatically. No interaction needed — safe commands are approved, everything else goes through the normal permission prompt.

As a CLI tool:

```bash
safe-chains "ls -la | head -5"    # exit 0 = safe
safe-chains "rm -rf /"            # exit 1 = unsafe
```

Every allowed command is classified as `inert`, `safe-read`, or `safe-write`. Use `--level` to set a threshold:

```bash
safe-chains --level inert "cargo test"       # exit 1 (safe-read > inert)
safe-chains --level safe-read "cargo test"   # exit 0
```

## Contributing

Found a safe command safe-chains should support? [Submit an issue.](https://github.com/michaeldhopkins/safe-chains/issues/new?template=command-request.yml)

See the [documentation](https://www.michaeldhopkins.com/docs/safe-chains/contributing.html) for how to add commands.

---

Copyright 2026 Michael Hopkins
