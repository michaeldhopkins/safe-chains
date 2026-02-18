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
cargo install --path .
```

Updates the installed binary.

## Development

- When adding a new command handler: add the handler in the appropriate `src/handlers/` module, register in `src/handlers/mod.rs` dispatch, add `#[cfg(test)]` tests covering both allow and deny, run the test suite and clippy
- Do not add comments to code
- All files must end with a newline
