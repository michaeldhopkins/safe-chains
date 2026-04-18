# Usage

## Claude Code (hook mode)

With the hook configured, safe-chains reads JSON from stdin and responds with a permission decision. No arguments needed.

## CLI mode

Pass a command as a positional argument. Exit code 0 means safe, exit code 1 means unsafe.

```bash
safe-chains "ls -la | head -5"    # exit 0 = safe
safe-chains "rm -rf /"            # exit 1 = unsafe
```

Commands containing shell operators (`&&`, `|`, `;`) are split into segments. Each segment is validated independently.
