# safe-chains

Agentic coding tools prompt you dozens of times per session for commands like 
- `git log --oneline | head -20`
- `cargo test && cargo clippy -- -D warnings`
- or even `find src -name "*.rs" -exec grep -l "TODO" {} \; | sort | while read f; do echo "=== $f ==="; grep -n "TODO" "$f"; done`.

You approve them all, and eventually stop reading the prompts, which is exactly when a destructive command slips through.

**safe-chains** parses these commands (pipes, chains, loops, subshells, nested wrappers) and approves only when every segment is verifiably safe. Now, you only get prompted to approve a command when something interesting comes along.

safe-chains covers {{#include includes/command-count.md}} commands with flag-level validation, compound command parsing, and recursive subshell expansion, all deterministically, not based on a model's guess like with Claude's auto mode.

## How it works

With your agent harness configured to run safe-chains from a hook, each Bash command is analyzed and gets a decision response.

Or, just run safe-chains yourself in your terminal to learn about a command. It's fun!
```bash
safe-chains "ls -la | head -5"    # exit 0 = safe
safe-chains "rm -rf /"            # exit 1 = unsafe
```

## Getting started

- [Installation](installation.md)
- [Configuration](configuration.md)
- [Overview](how-it-works.md)
- [Custom Commands](custom-commands.md)
