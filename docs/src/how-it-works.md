# How It Works

## Built-in rules

safe-chains knows {{#include includes/command-count.md}} commands. For each one it validates specific subcommands and flags, allowing `git log` but not `git push`, allowing `sed 's/foo/bar/'` but not `sed -i`.

## Parsing example

Take this command from the introduction:

```bash
find src -name "*.rs" -exec grep -l "TODO" {} \; | sort | while read f; do echo "=== $f ==="; grep -n "TODO" "$f"; done
```

Normally, you would be prompted to run this by your agent, and you would have to run some sort of auto- or permission-skipping mode to not be prompted, which could allow anything to be run.

Running [from a hook](#installation.md), safe-chains parses this and validates every leaf:

1. **Pipeline segment 1:** `find src -name "*.rs" -exec grep -l "TODO" {} \;`
   - `find` is allowed with positional predicates
   - `-exec` triggers delegation: the inner command `grep -l "TODO" {}` is extracted and validated separately
   - `grep -l` passes (`-l` is an allowed flag)
2. **Pipeline segment 2:** `sort` passes (safe with any arguments)
3. **Pipeline segment 3:** `while read f; do ...; done` is a compound command, parsed recursively:
   - `read f` passes (shell builtin)
   - `echo "=== $f ==="` passes
   - `grep -n "TODO" "$f"` passes (`-n` is an allowed flag)

Every leaf is safe, so the entire command is auto-approved without over-extending permissions to the agent.

## Interaction with approved commands

safe-chains runs as a pre-hook. If it approves, Claude Code skips the prompt. If it doesn't recognize the command, Claude Code's normal permission flow takes over (checking your `Bash(...)` patterns in settings, or prompting).

Where this gets interesting is chained commands. Claude Code matches approved patterns against the full command string. If you approved `Bash(cargo test:*)` and Claude runs `cargo test && ./generate-docs.sh`, Claude Code won't match — the full string isn't just `cargo test`.

safe-chains splits the chain and checks each segment independently. `cargo test` passes built-in rules. `./generate-docs.sh` matches `Bash(./generate-docs.sh:*)` from your settings. Both segments covered, chain auto-approved.

Once safe-chains is handling your safe commands, most of your existing approved patterns are redundant. Strip them down to project-specific scripts and tools safe-chains doesn't know about — or write a [Custom Command](custom-commands.md) for those scripts and let safe-chains validate them with the same flag-level rules it applies to built-ins. See [Cleaning up approved commands](configuration.md#cleaning-up-approved-commands).

For example, given `cargo test && npm run build && ./generate-docs.sh`:

- `cargo test` passes built-in rules
- `npm run build` matches `Bash(npm run:*)` from settings
- `./generate-docs.sh` matches `Bash(./generate-docs.sh:*)` from settings
