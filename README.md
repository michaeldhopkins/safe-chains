# `safe-chains`

A command safety checker that auto-allows safe, read-only bash commands without prompting. Works as a Claude Code pre-hook, a CLI tool, or an OpenCode plugin.

When an agentic tool wants to run a bash command, safe-chains checks if every segment of the command is safe. If so, the command runs without asking for permission. If any segment is unsafe, the normal permission flow takes over. Commands in piped chains, `&&`, and `;` sequences are each validated independently.

See [COMMANDS.md](COMMANDS.md) for the full list of supported commands.

## Installation

Requires [Rust](https://rustup.rs/) to `cargo install` the binary.

```bash
git clone git@github.com:michaeldhopkins/claude-safe-chains.git ~/workspace/claude-safe-chains
cd ~/workspace/claude-safe-chains
./install.sh
```

This builds the `safe-chains` binary into `~/.cargo/bin/` and outputs the hook configuration you need to add to `~/.claude/settings.json`:

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

If the hook is already configured, the script will skip this step.

You'll need to restart your Claude Code sessions to use the new hook, but once you do this, you'll be able to update `safe-chains` and benefit from the new version right away.

## Usage

### Claude Code (hook mode)

With the hook configured above, safe-chains reads JSON from stdin and responds with a permission decision. No arguments needed.

### CLI mode

Pass a command as a positional argument. Exit code 0 means safe, exit code 1 means unsafe.

```bash
safe-chains "ls -la | head -5"    # exit 0 = safe
safe-chains "rm -rf /"            # exit 1 = unsafe
```

### OpenCode

Copy `opencode-plugin.js` to `.opencode/plugins/` in your project (requires `safe-chains` in PATH).

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
