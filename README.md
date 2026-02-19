# `safe-chains`

A Claude Code pre-hook that auto-allows safe, read-only bash commands without prompting. This is a great hook for people who are sick of approving `Git -C ...`, `for...do...end` and the like dozens of times every time they work on a new project. This will also auto-approve piped commands, which Claude Code either will ask about every time or only allow approving "similar" commands that somehow never match future commands. Bonus: because you'll hardly need it, your approved commands list in settings will be shorter and more interesting. Stay frosty!

When Claude Code wants to run a bash command, this hook intercepts it and checks if every segment of the command is safe. If so, the command runs without asking for permission. If any segment is unsafe, the normal permission flow takes over. Commands in piped chains, `&&`, and `;` sequences are each validated independently.

See [COMMANDS.md](COMMANDS.md) for the full list of supported commands.

## Installation

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
