# CLAUDE.md

## Scope / trust model

safe-chains identifies a command by its **executable name and path** — never by
inspecting the binary's content or signature. It trusts that `cat` is GNU `cat`, that
`git` is git, and so on. Verifying that a binary is genuinely the tool it is named is
**out of scope** by design: this is a static, string-only classifier, not a sandbox or
an integrity checker.

Consequences to keep in mind (do not try to "fix" these by reading the filesystem — a
static classifier must not, and it would be TOCTOU-racy anyway):

- A `$PATH` shadow or a planted binary named after a safe tool is classified as that
  tool. (Mitigation the capability engine applies: a resolvable name reached via a
  *non-standard path* — `./cat`, `/tmp/cat` — is worst-cased; a bare name resolved via
  `$PATH`, and standard bin paths, are trusted.)
- Symlinks are not followed: a path is classified by its literal spelling, so a worktree
  symlink pointing outside the worktree reads as worktree-local.

If a threat model requires defending against a hostile checkout or `$PATH`, that belongs
to the harness/sandbox, not to safe-chains. See `docs/design/behavioral-taxonomy-engine.md`
§0.2.

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

One logical change per commit. Do not batch unrelated changes into one commit — we publish detailed changelogs and each entry should describe a single thing.

Examples of one commit:
- Adding a new command (TOML + tests)
- Adding missing flags to an existing command
- A bug fix to the parser
- A refactor of a handler

Examples of what should NOT be one commit:
- Adding a new command AND fixing an unrelated bug
- Refactoring a handler AND adding missing flags to a different command

## Versioning

One version bump per release (i.e. per push), not per commit. A "release" is the batch of commits being pushed together; intermediate commits in the stack must not bump `Cargo.toml`.

The bump level reflects the highest-impact change in the batch:
- **patch** if every commit in the batch is a bug fix
- **minor** if any commit adds a new command, flag, or feature

We are not ready for major bumps yet.

Fold the bump into the final feat/fix/refactor commit of the stack — do not create a separate `chore: bump version` commit. Run `cargo check` after bumping so `Cargo.lock` matches before pushing — CI uses `--locked`.

## Development

- Most commands are defined as TOML in `commands/*.toml`. See `commands/SAMPLE.toml` for the complete field reference — it documents every field type, when to use each one, and how they compose. Always check SAMPLE.toml before adding a new field type to ensure you aren't duplicating what existing fields already cover.
- When adding a new command: research the command first, then add it to the appropriate `commands/*.toml` file with a `description` field, run the test suite, clippy, and `./generate-docs.sh`. New `*.toml` files under `commands/` are auto-discovered by `build.rs`; no explicit registration needed.
- When adding a new TOML field type: design and thoroughly test the generic handler in `src/registry/build.rs` before using it in any data file. Add comprehensive tests covering every edge case. Update `commands/SAMPLE.toml` with documentation for the new field.
- Commands that need custom Rust validation (curl headers, perl AST, fzf --bind parsing) use `handler = "name"` in TOML and a Rust function in `src/handlers/`. This is a last resort — most commands can be expressed declaratively.
- Do not add comments to code
- All files must end with a newline

## Targets and hooks

Before touching any `src/targets/*.rs` (per-tool hook integration) or the
context the hook surfaces to agents, read `HARNESS-BEHAVIORS.md`. It is the
consolidated reference for how each harness drives its PreToolUse hook: the
decision-contract per tool (field name and accepted values — getting this wrong
fails *silently*), the abstain/timing model, and which targets support
`additionalContext`.

When a harness's own docs disagree with `HARNESS-BEHAVIORS.md`, the harness
wins: update both the relevant `targets/*.rs` and `HARNESS-BEHAVIORS.md` to
match, in the same change.

## When to use a Rust handler

Handlers are for **logic**, not **data**. A handler exists to make a routing or validation decision the declarative TOML shape can't express — not to hold a pile of allowed flags. If you find yourself writing `static FOO_FLAGS: WordSet = WordSet::new(&[...])` inside `src/handlers/`, you have data in the wrong place.

Gold standard: `src/handlers/ruby/bundle.rs::check_bundle_exec` — a few lines of pure dispatch with zero hardcoded data. The recently-rewritten `src/handlers/tilt.rs` is the clean example for handlers that want both subs and a fallback grammar:

```rust
pub fn check_tilt(tokens: &[Token]) -> Verdict {
    if let Some(verdict) = registry::try_sub_dispatch("tilt", tokens) {
        return verdict;
    }
    registry::try_fallback_grammar("tilt", tokens).unwrap_or(Verdict::Denied)
}
```

All flags, sub names, and shape predicates live in `commands/tools/tilt.toml`. The handler is logic only.

How to apply:
- Allowed sub names → `[[command.sub]]` blocks in TOML, dispatched via `registry::try_sub_dispatch(cmd_name, tokens)`.
- Alternate grammar engaged when no sub matches → `[command.fallback]` block in TOML, dispatched via `registry::try_fallback_grammar(cmd_name, tokens)`.
- A predicate over the first positional (e.g. "must look like a path") → `positional_shape = "path"` on the fallback. Adding new shapes is a one-line `PositionalShape` enum addition plus a match arm in `policy::PositionalShape::matches()`.

If a handler still wants hardcoded data, that means the TOML schema is missing an expressive primitive. Extend the schema (`TomlFallback`, `PositionalShape`, etc.) before adding more `WordSet` constants. The TOML field name and the corresponding Rust logic should be analogous so the data/logic mapping is obvious — `[command.fallback]` ↔ `try_fallback_grammar()`, `positional_shape` ↔ `PositionalShape::matches()`.

## Researching a new command

Before adding a command, research it and write a `description` field that serves as a safety analysis. The description is NOT a summary of what we support — it's an independent assessment of the command's behavior and risk profile, as if written by a security researcher who doesn't know how we'll use the report.

The description should cover:
- What the command does (one sentence)
- Which operations are read-only, which modify state, which touch the network, which execute arbitrary code
- Subtleties: subcommands that delegate to inner commands, flags that change behavior from safe to unsafe, read vs write modes on the same subcommand
- Project velocity: how often does it release? Is the flag surface stable or fast-moving? Look this up — don't guess.

Do NOT reference safe-chains internals (SafeWrite, SafeRead, Inert, handlers, allowlists) in the description. Describe the command itself.

Use `candidate = true` on subcommands that were considered but deliberately not approved. This records the decision so future contributors don't re-evaluate the same commands.

### Research the latest upstream version, not the local install

Whenever you research or revise a command, target the **latest released** version of the upstream tool, not whatever happens to be installed on the current machine. Check the project's GitHub releases, npm registry, crates.io, or official docs to confirm the current version, and read the latest reference for that version. If the local install and the online docs disagree, follow the online docs — local installs drift behind quickly and the goal is for safe-chains to match what an up-to-date user actually runs.

Record the version you researched in the `researched_version` field on `[[command]]` (free-form string — see SAMPLE.toml for forms). This field is internal-only (not rendered in docs) but it's the tripwire for the next person re-researching: they can see whether the current upstream is meaningfully ahead and what to diff against.

Backfill is not feasible for older TOMLs that pre-date the field. The right time to fill it in is the next time the command is researched — leave it absent until then rather than guess.

When safe-chains' classification differs from upstream behavior because the upstream changed, the upstream wins: treat the divergence as a follow-up to update our TOML, not a reason to keep our older shape.

### Obsolete entries are fine if they stayed safe

If a subcommand, flag, or task name was removed or renamed in a newer upstream version, you do NOT have to remove it from the allowlist — keep it as long as it cannot become **unsafe** in the latest researched version. A removed task that is now a no-op (or "task not found" error) carries no risk and helps users on older codebases. The bar for removing an obsolete entry is: "the same name now does something unsafe upstream." A short note in the description acknowledging the historical entries (e.g. "db:structure:load/dump date to pre-Rails-6 and are no-ops on current Rails") is preferable to silently dropping them.

## Documentation style

Doc strings in `command_docs()` must only describe what is **allowed**. This is an allowlist-only program.

- Never use: "denied", "blocked", "rejected", "forbidden", "dangerous", "unsafe", "not allowed", "Guarded"
- Never say "no flags", "no arguments", "no extra flags"
- Instead of "X denied" → just omit it (unlisted = not allowed)
- Instead of "No flags allowed" → "Bare invocation allowed." or just list the subcommands
- Instead of "Guarded: fmt (--check only)" → "fmt (requires --check)"
- Don't say "explicit flag allowlist" — the whole program is an allowlist, this is redundant
