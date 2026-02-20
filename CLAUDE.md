# CLAUDE.md

## Testing

```bash
cargo test
```

All tests must pass before committing.

## Linting

```bash
cargo clippy -- -D warnings
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
