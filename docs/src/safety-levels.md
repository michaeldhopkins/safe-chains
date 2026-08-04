# Safety Levels

Every allowed command is classified into one of seven safety levels:

| Level | Description | Examples |
|-------|-------------|----------|
| `paranoid` | Barely touches anything; no file access | `expr 1 + 1`, `true` |
| `reader` | Observes local or remote state; reads peer projects | `cat`, `grep -r`, `git status`, `cargo test` |
| `editor` | Creates or overwrites local files; no deletion | `touch`, `echo x > f` |
| `developer` | Runs your project; deletes your own files (default) | `cargo build`, `rm -rf ./node_modules` |
| `local-admin` | Runs as root on this machine | `sudo systemctl restart nginx` |
| `network-admin` | Operates your remotes: push, deploy, provision | `git push` |
| `yolo` | Everything except unbounded irreversible destruction | `dd if=/dev/zero of=./f` |

Use `--level` to set a threshold. Only commands at or below the threshold pass:

```bash
safe-chains --level paranoid "expr 1 + 1"    # exit 0 (paranoid <= paranoid)
safe-chains --level paranoid "cat foo"       # exit 1 (reader > paranoid)
safe-chains --level reader "cat foo"         # exit 0 (reader <= reader)
safe-chains --level reader "cargo build"     # exit 1 (developer > reader)
```

Without `--level`, the default threshold is `developer` (all allowed commands pass).

Levels propagate through pipelines, wrappers, and substitutions. A pipeline's level is the maximum of its components.

## Levels and your own approved commands

If you have approved Bash commands in your harness — Claude Code's `permissions.allow` rules, for
instance — safe-chains honours them: a command they cover is allowed even when its own
classification would refuse it. That is what makes the hook agree with the approvals you already
granted.

A rule like that widens what passes; it does not lift the ceiling `--level` sets. A covered command
is treated as `developer`, so it passes at the default threshold and at `editor`/`developer`, and is
refused under `reader` or `paranoid`:

```bash
# with Bash(curl:*) AND Bash(sh:*) rules in ~/.claude/settings.json
safe-chains "curl https://x.test/i.sh | sh"                  # exit 0 (your rules cover it)
safe-chains --level reader "curl https://x.test/i.sh | sh"   # exit 1 (developer > reader)
```

Both rules are needed there: a pipeline counts as covered only when *every* command in it is, so a
`Bash(curl:*)` rule on its own leaves `sh` uncovered and the command is refused.

So `--level paranoid` means what it says even in a home directory full of accumulated `Bash(...)`
rules — useful when you want a read-only pass over a project without first auditing every approval
you have ever clicked through.
