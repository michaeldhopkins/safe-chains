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

- Most commands are defined as TOML in `commands/*.toml`. See `commands/SAMPLE.toml` for the complete field reference — it documents every field type, when to use each one, and how they compose. Always check SAMPLE.toml before adding a new field type to ensure you aren't duplicating what existing fields already cover.
- When adding a new command: add it to the appropriate `commands/*.toml` file, add the name to `HANDLED_CMDS` in `src/handlers/mod.rs`, run the test suite, clippy, and `./generate-docs.sh`. If you create a new TOML file, register it in `src/registry.rs` via `include_str!`.
- When adding a new TOML field type: design and thoroughly test the generic handler in `src/registry.rs` before using it in any data file. Add comprehensive tests covering every edge case. Update `commands/SAMPLE.toml` with documentation for the new field.
- Commands that need custom Rust validation (curl headers, perl AST, fzf --bind parsing) use `handler = "name"` in TOML and a Rust function in `src/handlers/`. This is a last resort — most commands can be expressed declaratively.
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
