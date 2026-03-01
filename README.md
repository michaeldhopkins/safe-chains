# `safe-chains`

A command safety checker that auto-allows safe, read-only bash commands without prompting. Works as a Claude Code pre-hook, a CLI tool, or an OpenCode plugin.

When an agentic tool wants to run a bash command, safe-chains checks if every segment of the command is safe. If so, the command runs without asking for permission. If any segment is unsafe, the normal permission flow takes over. Commands in piped chains, `&&`, and `;` sequences are each validated independently.

For chained commands, safe-chains also reads your Claude Code settings to approve segments you've already permanently approved but that Claude Code can't match because the command contains shell operators. See [Settings-aware chain approval](#settings-aware-chain-approval).

See [COMMANDS.md](COMMANDS.md) for the full list of supported commands.

## Install

### Pre-built binary

Download from [GitHub Releases](https://github.com/michaeldhopkins/safe-chains/releases/latest). Binaries are available for macOS (Apple Silicon and Intel) and Linux (x86_64 and aarch64).

```bash
# Example for macOS Apple Silicon:
curl -L https://github.com/michaeldhopkins/safe-chains/releases/latest/download/safe-chains-aarch64-apple-darwin.tar.gz | tar xz
mv safe-chains ~/.cargo/bin/   # or anywhere in your PATH
```

### With Cargo

Requires [Rust](https://rustup.rs/).

```bash
cargo install safe-chains
```

### From source

```bash
git clone git@github.com:michaeldhopkins/safe-chains.git
cd safe-chains
cargo install --path .
```

## Configure

### Claude Code

Add this to `~/.claude/settings.json`:

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

Restart your Claude Code sessions to activate the hook. Once configured, updating the `safe-chains` binary takes effect immediately.

### OpenCode

Copy `opencode-plugin.js` from this repo to `.opencode/plugins/` in your project. Requires `safe-chains` in PATH.

## Usage

### Claude Code (hook mode)

With the hook configured above, safe-chains reads JSON from stdin and responds with a permission decision. No arguments needed.

### CLI mode

Pass a command as a positional argument. Exit code 0 means safe, exit code 1 means unsafe.

```bash
safe-chains "ls -la | head -5"    # exit 0 = safe
safe-chains "rm -rf /"            # exit 1 = unsafe
```

## How it works

### Built-in rules

safe-chains knows 130+ read-only commands (`grep`, `cat`, `ls`, `jq`, ...) that are always safe with any arguments. For 50+ additional tools (`git`, `cargo`, `npm`, `docker`, ...), it validates specific subcommands and flags—allowing `git log` but not `git push`, allowing `sed 's/foo/bar/'` but not `sed -i`.

Commands containing shell operators (`&&`, `|`, `;`) are split into segments, and each segment is validated independently. Segments containing output redirection (`>`), input redirection (`<`), backticks, or command substitution (`$(...)`) are always rejected.

### Settings-aware chain approval

`safe-chains` lets you significantly tighten up your approved bash commands and also makes your remaining allowed commands more useful. When a chain doesn't fully pass built-in rules, safe-chains reads your Claude Code settings and checks each segment independently against your approved patterns. For example, given `cargo test && npm run build && ./generate-docs.sh`:

- `cargo test`—passes built-in rules
- `npm run build`—matches `Bash(npm run *)` from settings
- `./generate-docs.sh`—matches `Bash(./generate-docs.sh)` from settings

All segments covered, so the chain is auto-approved. If any segment fails both checks, safe-chains makes no decision and Claude Code prompts as usual.

**Settings files read:**

| File | Source |
|------|--------|
| `~/.claude/settings.json` | Global settings |
| `$CLAUDE_PROJECT_DIR/.claude/settings.json` | Project settings |
| `$CLAUDE_PROJECT_DIR/.claude/settings.local.json` | Local project settings (gitignored) |

### Guardrails

Segments with `>`, `<`, `` ` ``, or `$(...)` outside of quotes are never approved via settings, even if a pattern matches. This prevents `Bash(./script *)` from approving `./script > /etc/passwd`.

Glob patterns are matched per-segment. A pattern matching one segment cannot cause another segment in the chain to be approved.

Missing, unreadable, or malformed settings files are silently skipped—fewer patterns means more conservative behavior.

Broad patterns like `Bash(bash *)` will approve nested commands without recursive validation. If you have `Bash(bash *)` in your settings and a segment is `bash -c "safe-cmd && evil-cmd"`, the settings match takes precedence over the built-in shell handler's recursive check. This matches Claude Code's own behavior for approved patterns.

When matching allowed commands from settings, `safe-chains` splits first, then matches each segment in isolation, so `*`/`:*` doesn't leak.

## Developing

```bash
cargo test
cargo clippy -- -D warnings
```

Adding a new command:

1. Add constants and handler function in the appropriate `src/handlers/` module
2. Register it in the dispatch match in `src/handlers/mod.rs`
3. Add `#[cfg(test)]` tests in the handler module covering both allow and deny cases
4. Run `cargo test` and `cargo clippy -- -D warnings`
5. Run `./generate-docs.sh` to regenerate COMMANDS.md
6. Run `cargo install --path .` to update the installed binary

---

Copyright 2026 Michael Hopkins
