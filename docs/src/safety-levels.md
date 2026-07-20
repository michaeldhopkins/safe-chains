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
