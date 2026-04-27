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

Regenerates COMMANDS.md, builds and deploys the documentation site, and updates the installed binary.

## Commits

One logical change per commit. Each commit gets its own semver bump. Do not batch unrelated changes into one commit — we publish detailed changelogs and each entry should describe a single thing.

Examples of one commit:
- Adding a new command (TOML + HANDLED_CMDS + tests)
- Adding missing flags to an existing command
- A bug fix to the parser
- A refactor of a handler

Examples of what should NOT be one commit:
- Adding a new command AND fixing an unrelated bug
- Refactoring a handler AND adding missing flags to a different command

Bump the version in `Cargo.toml` with each commit using semver: patch for bug fixes, minor for new commands/features, major for breaking changes.

## Development

- Most commands are defined as TOML in `commands/*.toml`. See `commands/SAMPLE.toml` for the complete field reference — it documents every field type, when to use each one, and how they compose. Always check SAMPLE.toml before adding a new field type to ensure you aren't duplicating what existing fields already cover.
- When adding a new command: research the command first, then add it to the appropriate `commands/*.toml` file with a `description` field, add the name to `HANDLED_CMDS` in `src/handlers/mod.rs`, run the test suite, clippy, and `./generate-docs.sh`. If you create a new TOML file, register it in `src/registry.rs` via `include_str!`.
- When adding a new TOML field type: design and thoroughly test the generic handler in `src/registry.rs` before using it in any data file. Add comprehensive tests covering every edge case. Update `commands/SAMPLE.toml` with documentation for the new field.
- Commands that need custom Rust validation (curl headers, perl AST, fzf --bind parsing) use `handler = "name"` in TOML and a Rust function in `src/handlers/`. This is a last resort — most commands can be expressed declaratively.
- Do not add comments to code
- All files must end with a newline

## Researching a new command

Before adding a command, research it and write a `description` field that serves as a safety analysis. The description is NOT a summary of what we support — it's an independent assessment of the command's behavior and risk profile, as if written by a security researcher who doesn't know how we'll use the report.

The description should cover:
- What the command does (one sentence)
- Which operations are read-only, which modify state, which touch the network, which execute arbitrary code
- Subtleties: subcommands that delegate to inner commands, flags that change behavior from safe to unsafe, read vs write modes on the same subcommand
- Project velocity: how often does it release? Is the flag surface stable or fast-moving? Look this up — don't guess.

Do NOT reference safe-chains internals (SafeWrite, SafeRead, Inert, handlers, allowlists) in the description. Describe the command itself.

Use `candidate = true` on subcommands that were considered but deliberately not approved. This records the decision so future contributors don't re-evaluate the same commands.

## Documentation style

Doc strings in `command_docs()` must only describe what is **allowed**. This is an allowlist-only program.

- Never use: "denied", "blocked", "rejected", "forbidden", "dangerous", "unsafe", "not allowed", "Guarded"
- Never say "no flags", "no arguments", "no extra flags"
- Instead of "X denied" → just omit it (unlisted = not allowed)
- Instead of "No flags allowed" → "Bare invocation allowed." or just list the subcommands
- Instead of "Guarded: fmt (--check only)" → "fmt (requires --check)"
- Don't say "explicit flag allowlist" — the whole program is an allowlist, this is redundant
