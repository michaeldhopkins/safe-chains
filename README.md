# `safe-chains`

A command safety checker that auto-allows safe bash commands without prompting. Works as a Claude Code pre-hook, a CLI tool, or an OpenCode plugin (experimental).

When an agentic tool wants to run a bash command, safe-chains checks if every segment of the command is safe. If so, the command runs without asking for permission. If any segment is unsafe, the normal permission flow takes over. Commands in piped chains, `&&`, and `;` sequences are each validated independently.

For chained commands, safe-chains also reads your Claude Code settings to approve segments you've already permanently approved but that Claude Code can't match because the command contains shell operators. See [Settings-aware chain approval](#settings-aware-chain-approval).

See [COMMANDS.md](COMMANDS.md) for the full list of supported commands. See [SECURITY.md](SECURITY.md) for security properties and threat model.

## Install

### Homebrew (macOS)

```bash
brew install michaeldhopkins/tap/safe-chains
```

This installs the binary, man page, shell completions, and the OpenCode plugin. After installing, run `safe-chains --setup` to configure the Claude Code hook (see below).

### Pre-built binary

Download signed, notarized binaries from [GitHub Releases](https://github.com/michaeldhopkins/safe-chains/releases/latest). Available for macOS (Apple Silicon and Intel) and Linux (x86_64 and aarch64).

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

Run `safe-chains --setup` to automatically configure the hook in `~/.claude/settings.json`. Or manually add this to `~/.claude/settings.json`:

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

### OpenCode (experimental)

OpenCode's plugin API does not yet support auto-allowing commands. The [`permission.ask` hook](https://github.com/anomalyco/opencode/issues/7006) that would enable this is defined in the SDK but never triggered. OpenCode runs permission checks *before* plugin hooks, so `tool.execute.before` cannot bypass the permission prompt for safe commands.

**Recommended setup:** Generate OpenCode `permission.bash` rules from safe-chains' command list:

```bash
safe-chains --opencode-config
```

This reads your existing `opencode.json` (if present), merges safe-chains permission rules into it, and prints the result. Review the output, then apply:

```bash
safe-chains --opencode-config > opencode.json
```

The generated rules are coarser than safe-chains' flag-level validation — they allow any flags for a safe subcommand (e.g., `"git status *": "allow"`), while safe-chains validates specific flags. Shell wrappers (`bash`, `env`, `xargs`, etc.) are excluded since their safety depends on the inner command.

**Plugin (optional):** The included `opencode-plugin.js` blocks commands safe-chains considers unsafe. With `"*": "allow"` in your permission config, the plugin acts as a safety net — all commands are auto-allowed, and the plugin blocks unsafe ones with an error. Without `"*": "allow"`, the plugin adds a second layer of restriction on top of OpenCode's permission prompts.

```bash
mkdir -p .opencode/plugins
cp $(brew --prefix)/share/safe-chains/opencode-plugin.js .opencode/plugins/
```

Requires `safe-chains` in PATH.

## Usage

### Claude Code (hook mode)

With the hook configured above, safe-chains reads JSON from stdin and responds with a permission decision. No arguments needed.

### CLI mode

Pass a command as a positional argument. Exit code 0 means safe, exit code 1 means unsafe.

```bash
safe-chains "ls -la | head -5"    # exit 0 = safe
safe-chains "rm -rf /"            # exit 1 = unsafe
```

#### Safety levels

Every allowed command is classified into one of three safety levels:

| Level | Description | Examples |
|-------|-------------|----------|
| `inert` | Pure read/display, no code execution | `cat`, `grep`, `ls`, `git log` |
| `safe-read` | Executes code but read-only | `cargo test`, `rspec`, `npm test` |
| `safe-write` | May modify files but considered safe | `cargo build`, `go build` |

Use `--level` to set a threshold. Only commands at or below the threshold pass:

```bash
safe-chains --level inert "cat foo"          # exit 0 (inert ≤ inert)
safe-chains --level inert "cargo test"       # exit 1 (safe-read > inert)
safe-chains --level safe-read "cargo test"   # exit 0 (safe-read ≤ safe-read)
safe-chains --level safe-read "cargo build"  # exit 1 (safe-write > safe-read)
```

Without `--level`, the default threshold is `safe-write` (all allowed commands pass). `--help` and `--version` on any command are always `inert`. Levels propagate through pipelines, wrappers, and substitutions — a pipeline's level is the maximum of its components.

## How it works

### Built-in rules

safe-chains knows 420+ commands. Of these, 110+ (`grep`, `cat`, `ls`, `jq`, ...) are safe with any arguments. For 300+ additional tools (`git`, `cargo`, `npm`, `docker`, ...), it validates specific subcommands and flags—allowing `git log` but not `git push`, allowing `sed 's/foo/bar/'` but not `sed -i`.

Commands containing shell operators (`&&`, `|`, `;`) are split into segments. Shell compound commands (`for`/`while`/`until` loops and `if`/`elif`/`else` conditionals) are parsed into an AST and each leaf command is validated recursively, supporting arbitrary nesting depth. Simple segments are validated independently. Output redirection (`>`, `>>`) is only allowed to `/dev/null`. Input redirection (`<`) is only allowed from `/dev/null`. Here-strings (`<<<`) and here-documents (`<<`, `<<-`) are allowed. Backticks and command substitution (`$(...)`) are recursively validated.

Found a safe command safe-chains should support? [Submit an issue.](https://github.com/michaeldhopkins/safe-chains/issues/new?template=command-request.yml)

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

### Tip: prefer CLI tools over Python

With compound command support, most commands that trigger approval prompts will be `python3` invocations. Consider instructing Claude to prefer safe CLI tools (`jq`, `grep`, `awk`, `sed`, `sort`, `curl`, etc.) over `python3` for data processing. Add something like this to your project's `CLAUDE.md`:

```
Prefer CLI tools (jq, grep, awk, sed, sort, curl) over python3 for data processing.
```

This reduces approval prompts without weakening security—when `python3` does come up for approval, it's more likely to be something worth paying attention to.

## Developing

```bash
cargo test
cargo clippy -- -D warnings
```

Adding a new command:

Most commands are defined as TOML in the `commands/` directory. See [`commands/SAMPLE.toml`](commands/SAMPLE.toml) for a documented reference of every supported field and when to use each one.

1. Add the command to the appropriate `commands/*.toml` file (or create a new one)
2. If you created a new file, register it in `src/registry.rs` via `include_str!`
3. Add the command name to `HANDLED_CMDS` in `src/handlers/mod.rs`
4. Run `cargo test` and `cargo clippy -- -D warnings`
5. Run `./generate-docs.sh` to regenerate COMMANDS.md
6. Run `cargo install --path .` to update the installed binary

For commands that need custom validation logic (rare — see SAMPLE.toml for guidance), add a Rust handler in `src/handlers/` and reference it with `handler = "name"` in the TOML.

---

Copyright 2026 Michael Hopkins
