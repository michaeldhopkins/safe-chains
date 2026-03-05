# CLAUDE.md

## Testing

```bash
cargo test
```

All tests must pass before committing.

## Linting

```bash
cargo clippy -- -D warnings
cargo deny check licenses
```

Must pass with no warnings before committing.

## After changes

```bash
./generate-docs.sh
cargo install --path .
```

Regenerates COMMANDS.md and updates the installed binary.

## Development

- When adding a new command handler: add the handler in the appropriate `src/handlers/` module, register in `src/handlers/mod.rs` dispatch, add `command_docs()` entries, add `#[cfg(test)]` tests covering both allow and deny, run the test suite, clippy, and `./generate-docs.sh`
- Do not add comments to code
- All files must end with a newline
- Bump the version in `Cargo.toml` with each commit using semver: patch for bug fixes, minor for new commands/features, major for breaking changes

## Documentation style

Doc strings in `command_docs()` must only describe what is **allowed**. This is an allowlist-only program.

- Never use: "denied", "blocked", "rejected", "forbidden", "dangerous", "unsafe", "not allowed", "Guarded"
- Never say "no flags", "no arguments", "no extra flags"
- Instead of "X denied" → just omit it (unlisted = not allowed)
- Instead of "No flags allowed" → "Bare invocation allowed." or just list the subcommands
- Instead of "Guarded: fmt (--check only)" → "fmt (requires --check)"
- Don't say "explicit flag allowlist" — the whole program is an allowlist, this is redundant
