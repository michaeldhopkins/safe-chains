# safe-chains

Agentic coding tools prompt you dozens of times for commands like `git log --oneline | head -20`, `cargo test && cargo clippy -- -D warnings`, or even `find src -name "*.rs" -exec grep -l "TODO" {} \; | sort | while read f; do echo "=== $f ==="; grep -n "TODO" "$f"; done`. You approve them all, and eventually stop reading the prompts, which is exactly when a destructive command slips through.

safe-chains parses these commands (pipes, chains, loops, subshells, nested wrappers) and auto-approves them when every segment is verifiably safe. The prompts that remain are the ones worth reading.

It covers {{#include includes/command-count.md}} commands with flag-level validation, compound command parsing, and recursive subshell expansion, all deterministic, with no AI in the loop. If safe-chains isn't sure, it doesn't guess. It leaves the prompt for you.

safe-chains works as a Claude Code pre-hook, a CLI tool, or an OpenCode plugin.

## Getting started

- [Installation](installation.md)
- [Configuration](configuration.md)
- [Usage](usage.md)
- [Safety Levels](safety-levels.md)

## How it works

- [Overview](how-it-works.md)
- [Security](security.md)

## Command reference

- [Overview](commands/README.md) — glossary and category index
- [Adding Commands](contributing.md) — how to contribute new commands
