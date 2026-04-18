# How It Works

## Built-in rules

safe-chains knows 420+ commands. Of these, 110+ (`grep`, `cat`, `ls`, `jq`, ...) are safe with any arguments. For 300+ additional tools (`git`, `cargo`, `npm`, `docker`, ...), it validates specific subcommands and flags — allowing `git log` but not `git push`, allowing `sed 's/foo/bar/'` but not `sed -i`.

## Compound commands

Shell compound commands (`for`/`while`/`until` loops and `if`/`elif`/`else` conditionals) are parsed into an AST and each leaf command is validated recursively, supporting arbitrary nesting depth.

Output redirection (`>`, `>>`) is only allowed to `/dev/null`. Input redirection (`<`) is only allowed from `/dev/null`. Here-strings (`<<<`) and here-documents (`<<`, `<<-`) are allowed.

Backticks and command substitution (`$(...)`) are recursively validated.

## Settings-aware chain approval

When a chain doesn't fully pass built-in rules, safe-chains reads your Claude Code settings and checks each segment independently against your approved patterns.

For example, given `cargo test && npm run build && ./generate-docs.sh`:

- `cargo test` — passes built-in rules
- `npm run build` — matches `Bash(npm run *)` from settings
- `./generate-docs.sh` — matches `Bash(./generate-docs.sh)` from settings

All segments covered, so the chain is auto-approved.

## Guardrails

Segments with `>`, `<`, `` ` ``, or `$(...)` outside of quotes are never approved via settings, even if a pattern matches. This prevents `Bash(./script *)` from approving `./script > /etc/passwd`.
