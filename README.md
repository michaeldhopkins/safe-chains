# claude-safe-chains

A Claude Code pre-hook that auto-allows safe, read-only bash commands without prompting. Commands in piped chains, `&&`, and `;` sequences are each validated independently.

## What it does

When Claude Code wants to run a bash command, this hook intercepts it and checks if every segment of the command is safe. If so, the command runs without asking for permission. If any segment is unsafe, the normal permission flow takes over.

Handled commands:
- **Always safe**: find, grep, rg, cat, ls, head, tail, sort, jq, base64, xxd, etc.
- **git**: read-only subcommands (log, diff, show, status, fetch, etc.) with `-C` support
- **jj**: read-only subcommands (log, diff, show, status, op log, file show, config get)
- **gh**: read-only subcommands + GET-only API calls
- **bundle**: list/info/show/check + exec with safe targets (rspec, standardrb, etc.)
- **yarn/npm**: read-only info commands + yarn test
- **mise/asdf**: read-only query commands
- **gem/brew/cargo**: read-only subsets
- **timeout/time**: recursive validation of wrapped commands
- **xargs/bash -c**: recursive validation of inner commands

## Installation

```bash
git clone git@github.com:michaeldhopkins/claude-safe-chains.git ~/workspace/claude-safe-chains
cd ~/workspace/claude-safe-chains
./install.sh
```

This creates a symlink from `~/.claude/hooks/safe-chains.sh` to the repo's copy.

## Running tests

```bash
python3 test_safe_chains.py
```

## Adding a new command

1. Add the handler in `safe-chains.sh` (constants at the top, handler block in `is_safe()`)
2. Add test cases in `test_safe_chains.py` covering both allow and deny
3. Run `python3 test_safe_chains.py`
