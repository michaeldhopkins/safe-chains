# `safe-chains`

A command safety checker that auto-allows safe, read-only bash commands without prompting. Works as a Claude Code pre-hook, a CLI tool, or an OpenCode plugin.

When an agentic tool wants to run a bash command, safe-chains checks if every segment of the command is safe. If so, the command runs without asking for permission. If any segment is unsafe, the normal permission flow takes over. Commands in piped chains, `&&`, and `;` sequences are each validated independently.

See [COMMANDS.md](COMMANDS.md) for the full list of supported commands.

## Install

Requires [Rust](https://rustup.rs/).

### From crates.io

```bash
cargo install safe-chains
```

### From source

```bash
git clone git@github.com:michaeldhopkins/safe-chains.git
cd safe-chains
cargo install --path .
```

Both methods build the `safe-chains` binary and place it in `~/.cargo/bin/`.

### What about install.sh?

The repo includes an `install.sh` convenience script. All it does is run `cargo install --path .` and then check whether your `~/.claude/settings.json` already has the hook configured. If not, it prints the JSON snippet you need to add. It does not modify any files.

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
