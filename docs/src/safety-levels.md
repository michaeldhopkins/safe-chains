# Safety Levels

Every allowed command is classified into one of three safety levels:

| Level | Description | Examples |
|-------|-------------|----------|
| `inert` | Pure read/display, no code execution | `cat`, `grep`, `ls`, `git log` |
| `safe-read` | Executes code but read-only | `cargo test`, `rspec`, `npm test` |
| `safe-write` | May modify files but considered safe | `cargo build`, `go build` |

Use `--level` to set a threshold. Only commands at or below the threshold pass:

```bash
safe-chains --level inert "cat foo"          # exit 0 (inert <= inert)
safe-chains --level inert "cargo test"       # exit 1 (safe-read > inert)
safe-chains --level safe-read "cargo test"   # exit 0 (safe-read <= safe-read)
safe-chains --level safe-read "cargo build"  # exit 1 (safe-write > safe-read)
```

Without `--level`, the default threshold is `safe-write` (all allowed commands pass).

Levels propagate through pipelines, wrappers, and substitutions — a pipeline's level is the maximum of its components.
