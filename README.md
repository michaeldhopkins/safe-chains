# claude-safe-chains

A Claude Code pre-hook that auto-allows safe, read-only bash commands without prompting. Commands in piped chains, `&&`, and `;` sequences are each validated independently.

## What it does

When Claude Code wants to run a bash command, this hook intercepts it and checks if every segment of the command is safe. If so, the command runs without asking for permission. If any segment is unsafe, the normal permission flow takes over.

Handled commands:
- **Always safe**: grep, rg, cat, ls, head, tail, jq, base64, xxd, etc.
- **git**: read-only subcommands (log, diff, show, status, fetch, etc.) with `-C` support
- **jj**: read-only subcommands (log, diff, show, status, op log, file show, config get)
- **gh**: read-only subcommands + GET-only API calls
- **bundle**: list/info/show/check + exec with safe targets (rspec, standardrb, etc.)
- **yarn/npm**: read-only info commands + yarn test
- **mise/asdf**: read-only query commands
- **gem/brew/cargo**: read-only subsets
- **npx**: whitelisted packages (eslint, karma, @herb-tools/linter)
- **find/sed/sort**: safe by default, denied with dangerous flags (-delete, -exec, -i, -o)
- **timeout/time**: recursive validation of wrapped commands
- **xargs/bash -c**: recursive validation of inner commands

## Installation

```bash
git clone git@github.com:michaeldhopkins/claude-safe-chains.git ~/workspace/claude-safe-chains
cd ~/workspace/claude-safe-chains
./install.sh
```

This runs `cargo install --path .` to put the `safe-chains` binary in `~/.cargo/bin/`.

## Running tests

```bash
cargo test
```

## Linting

```bash
cargo clippy -- -D warnings
```

## Adding a new command

1. Add constants and handler function in the appropriate `src/handlers/` module
2. Register it in the dispatch match in `src/handlers/mod.rs`
3. Add `#[cfg(test)]` tests in the handler module covering both allow and deny cases
4. Run `cargo test` and `cargo clippy -- -D warnings`
5. Run `cargo install --path .` to update the installed binary
