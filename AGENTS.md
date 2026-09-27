# AGENTS.md

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

### Default to property/invariant tests, not example tests

This is a security classifier: a missed case is a bypass or a false-deny, and the same shape recurs
across dozens of commands. So when you fix a bug or add a feature, **the default is to write a
property/generative guard, not just an example test.** Reach for one almost every time — even when it
looks like a single-case issue, it usually isn't, and new commands get added that fit the pattern.

The habit, on every fix or feature:

1. **Ask "what's the invariant?"** — not "what input broke?" State the rule the bug violated (e.g.
   "a write-enabling flag must never allow an out-of-workspace write", "the four spellings of a
   valued flag classify identically", "a handler never panics on any input").
2. **Generalize it.** Prefer a corpus/table or an **enumeration over the real registry/handlers**
   (walk `commands/**/*.toml`, iterate `handler_docs()`) so *new* entries are covered automatically
   — not a hand-picked example that only guards today's case.
3. **Prove it's not vacuous.** Red→green: break the fix, watch the guard fail, restore. A guard that
   can't fail is theater.
4. **Fail closed and say what's uncovered.** If a guard skips cases (needs a positional it can't
   synthesize, etc.), skip *safely* and note it.

Why this pays: every such guard in this repo has either found *more* bugs or generalized — the
interpreter-escape corpus flushed both mlr `put` and sed `1e`; the no-panic fuzz caught a bare-command
panic; flag-form equivalence caught a false-deny class; the eval_safe behavioral guard tripped three
commands at once in its red demo. An example test proves one case; a property guard kills the class
and keeps it dead.

The existing guards live in `tests/`, `src/handler_property_tests.rs`, `src/registry/tests.rs`
(the `every_*` guards), and `src/engine/testgen.rs` (the level-algebra proptests). Read them before
adding a new one — extend a corpus/table where one already fits rather than starting a fresh test.

### Fuzzing (project specifics)

General fuzzing method — the two budgets, the seeding ladder, coverage, CI shape, the gotchas — is
in the `rust-fuzzing` skill. This section is only what is specific to safe-chains.

- **Targets** (`fuzz/fuzz_targets/`; each file's header states its properties in full):
  - `parse` — `is_safe_command` on arbitrary bytes: never panics, never hangs, across parse → CST →
    engine → handlers. It **discards the verdict**, so it finds availability bugs only.
  - `equivalence` — a semantics-preserving respelling (flag forms, env-var forms) cannot change the
    verdict.
  - `hook_envelope` — the `targets/*` hook I/O never panics and never emits a grant it was not
    asked for.
  - `explain_render` — the explanation describes the verdict that was enforced, and a command
    cannot forge a marker line or smuggle a control/bidi character into it.
  - `suggest_roundtrip` — every config `--suggest` generates parses back, and merging never drops
    what was there.
  - `level_monotonic` — a stricter level never approves what a looser one refused.
  - `config_load` — a repo `.safe-chains.toml` (the one attacker-placed input) never aborts the
    loader, loads NOTHING when it is not valid TOML, and loads deterministically.
  - `setup_merge` — `--setup` into an arbitrary existing settings file: on refusal the file is
    byte-identical, on success it still parses.
  - `path_admit` — the credential shield outranks every package-content read admit, and those
    admits never grant a write.
  - `gate_prefilter` — a declared path gate never skips a value its own judge would refuse.
- **Not fuzzed:** `docs.rs` (no security property) and `pathctx` (proptests cover it and shrink
  better for a pure function).
- **CI:** `fuzz-replay.yml` replays every target's cached corpus (`-runs=0`) on every push and PR —
  the gate. `fuzz.yml` is a non-gating burst on each push to `main` (and `workflow_dispatch`, which
  takes a larger `max_total_time` and also renders the `parse` coverage report): one job per target
  running `fuzz/burst.sh`: 180s of single-process mutation timed from when the corpus has loaded
  (libFuzzer's `-max_total_time` counts the load, which on a runner can exceed the whole budget),
  then `-merge=1` into the corpus, saved as `fuzz-corpus-<target>-<run>` only when the merge
  completed, which is the prefix the replay restores. The burst's check fails on a crash-,
  timeout- or oom- artifact, or on a burst step that failed without one. `tests/fuzz_targets_wired.rs`
  keeps both matrices equal to `fuzz/Cargo.toml`, holds the cache prefix and those two conditions
  in step, and refuses a `schedule:` trigger. Both fuzz workflows skip docs-only pushes
  (`**.md`, `docs/**`): nothing fuzzed reads either.
- **`fuzz/burst.sh` runs the same locally** (`fuzz/burst.sh <binary> <target> <seconds>`, from the
  repo root). Finds go to `fuzz/new/<target>`, so bursting targets one after another never merges
  one target's finds into the next; the committed `seed-*` inputs keep their names through the
  merge; `fuzz/dict/<target>.dict` is used when present. `tests/fuzz_burst.rs` holds those against
  a stand-in libFuzzer binary.
- **No nightly (retired 2026-09-26).** It ran 05:00 UTC: three 5h shards on `parse` plus 1h on each
  property target. Every real find came in a target's first days; after the first week of August it
  ran seven weeks without another, and its later red nights were job timeouts (`config_load`
  overrunning `timeout-minutes`), not findings. The burst fuzzes new code the day it lands, which is
  when finds happen. For a deeper run after a large change, dispatch `fuzz.yml` with a bigger budget.
- **Seeding is registry-derived.** `src/bin/gen_fuzz_corpus.rs` (feature `fuzz-gen`,
  `cargo run --bin gen-fuzz-corpus --features fuzz-gen`) reads `commands/**/*.toml` and emits the
  `examples_safe`/`examples_denied` invocations as seeds and the command/subcommand/flag vocabulary
  as a `-dict` for `parse`. New commands are covered automatically — **no hand-maintained fuzz
  corpus.** The burst's build job regenerates them each run and the `parse` burst merges them in.
  Generated `gen-*` seeds and `fuzz/dict/` are git-ignored. The property targets take other input
  domains (a flag value, an envelope, a config file), so the registry seeds do not apply to them.
- **Regression seeds are the one hand-kept input.** A minimized find is committed as `seed-*` in
  its target's corpus (`.gitignore` lets `seed-*` through for `parse` and `explain_render`), so the
  replay holds it down. `explain_render/seed-unclosed-brace-nest` is the 2026-09-26 timeout: an
  unclosed `{` nest was parsed once as a brace group and again as a command named `{`, doubling per
  level. The parser's guards for that class count work instead of timing it (`cst/budget.rs` holds
  the entry and step budgets; the tests in `cst/parse.rs` assert work linear in nesting and walk
  every committed command seed), so they hold on a loaded machine.
- **Measured coverage** (region, authored source): mutation corpus alone ~26%, registry seeds alone
  ~37%, **combined ~61%**. The two are complementary — seeds unlock the per-command grammars,
  mutation covers parser byte-paths.
- **Two kinds of coverage hole, and what each means:**
  - **Un-exampled commands** — a handler at ~0% (e.g. `glab`, `magick`, `sysctl`) almost always has
    NO `examples_safe`/`examples_denied`, so nothing seeds it and byte mutation never synthesizes
    `glab mr list`. Adding examples is the **cheapest coverage win** and doubles as a
    `toml_examples_match_dispatch` guard. Prefer this before anything fancier.
  - **Out-of-scope layers** — `targets/*` (hook-envelope I/O), `docs.rs`/`registry/docs.rs`, and
    `suggest.rs` sit at ~0% because `is_safe_command` never calls them. They are not blind spots of
    the `parse` target; reaching them needs a **new** target (a hook-envelope target for
    `targets/`). Don't chase these by seeding `parse`.
  - **`cst/explain.rs` + `cst/display.rs` no longer need a target of their own.** The `parse` fuzz
    target still does not reach them — it calls `is_safe_command` — so their FUZZ coverage is
    unchanged and this bullet is not a claim about the corpus. What changed is that the two
    property guards in `handler_property_tests.rs`, which already generate adversarial command
    strings, now also call the renderer.
    `arbitrary_command_strings_never_panic` and `classifier_terminates_on_adversarial_input` now
    run `explain(&line).render()` alongside `command_verdict`, so the renderer is held to the same
    never-panics / never-hangs / deterministic contract. That matters because the layer is not
    merely cosmetic: `--explain` is user-facing AND renders the hook's injected context, where a
    panic is a crash — which for a PreToolUse hook fails OPEN. Extending a corpus beat building a
    target; check that before reaching for new machinery on the layers above.

### Mutation testing (project specifics)

General method is in the `rust-mutation-testing` skill. This is what is specific to safe-chains.

- **Per change** (`.github/workflows/mutants.yml`, not gating): `--in-diff` on every PR and push to
  `main`, skipped with a warning above 30 selected mutants; and on each push to `main` one
  **rotating slice**, `--shard (run_number % 256)/256`. PR runs advance the same counter, so
  coverage is roughly one pass per 256 runs rather than exactly one per 256 pushes. There is no whole-tree sweep: ~5,300 mutants at ~19s of wall clock each is ~28 hours.
- **N = 256, measured 2026-09-27** on slice 128/256 (21 mutants in `engine/resolve.rs`): 6m45s
  locally at `-j2` on a loaded M3 (baseline 21s build + 45s test). The runner is slower; if slices
  approach the 20-minute job timeout, raise N rather than the timeout. The 30-mutant in-diff cap is
  from the same local rate and wants recalibrating from the first runner timings.
- **Test-bound, and the profile is the lever.** A mutant's rebuild is 2-5s; its test run is the full
  suite. The integration tests spawn the debug binary a few hundred times, and unoptimized each
  spawn spent ~0.4s parsing the registry TOML inside the dependencies. `.cargo/mutants.toml` selects
  the `mutants` profile (Cargo.toml), which optimizes dependencies only: suite 124s to 45s, rebuild
  cost unchanged. Restricting the test command was not an option: `cli_gate`, `path_policy` and the
  hook tests drive the real binary (`CARGO_BIN_EXE_safe-chains`), which is built from the mutated
  tree, so they catch mutants the lib tests do not.
- **The `fuzz-gen` feature is on** for mutation runs, or `gen-fuzz-corpus` is never built and all of
  its mutants read as MISSED. CI's test job runs its tests the same way.
- **Exclusion:** `gen-fuzz-corpus`'s `main` (binds the checkout path and prints counts; `generate`
  holds the logic and is tested). Reason in `.cargo/mutants.toml`.
- **Equivalent, not excluded:** `delete !` in `sed_cluster`'s `-f` arm (`consumes_next: !has`).
  `scan_sed` flags every `-f` spelling and `resolve_sed` worst-cases before the cluster parser runs,
  so the `ScriptFile` arm's word count is never observed. A slice that lands on it reports it.
- **Findings of the first slices (2026-09-27):** slice 128/256 had 5 MISSED of 20 viable (75%): no
  test pinned which word `sed -e`/`-l` consume, that a trailing `-l` fails closed, or that an unknown
  byte in a short cluster fails closed when the script itself is harmless (`sed -Q ./foo` was caught
  only because `./foo` is an unknown sed command). Four now have
  `sed_short_flags_consume_exactly_their_own_value`; the fifth is the equivalent one above.
  `gen-fuzz-corpus` was never built by `cargo test` and had no tests; it now has four, and a run
  over the file caught 27 of 28 viable (the 28th is the excluded `main`).
- **Hermeticity it surfaced:** cargo-mutants builds in a copy under `$TMPDIR`, and
  `the_path_policy_corpus_holds` used the checkout itself as the workspace under the real `$HOME`, so
  its baseline failed there (a sibling is `adjacent` only under `$HOME`). The test now builds
  `~/projects/safe-chains` and a peer under a HOME of its own.

## Linting

```bash
cargo clippy --all-targets --all-features -- -D warnings
cargo deny check licenses
```

Must pass with no warnings before committing.

**`--all-targets` is not optional.** Without it clippy checks only the library and binaries — every
`#[cfg(test)]` module and everything under `tests/` is invisible to the lint pass. That is most of
this repo: the property guards, the registry `every_*` sweeps and the corpus tests are where the
safety argument actually lives, and they went unlinted long enough to accumulate a backlog. The lint
config in `Cargo.toml` (`unwrap_used`, `collapsible-if`, `cognitive-complexity`, `too-many-lines`)
is written for that code too.

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

### Classify BEHAVIOR along the facets — not "read-only vs the rest"

safe-chains does **not** sort a command into "read-only" and "everything else". It classifies what
the command DOES along the behavioral taxonomy's axes (canonical spec:
`docs/design/behavioral-taxonomy-v1.4.md`; the enum of axes: `src/engine/facet.rs`), and the safety
level a command/subcommand earns is simply *where that profile lands*. "Read-only" is not a facet —
it flattens the axes that actually decide safety. Never research or reason about "the read-only
subcommands"; research the WHOLE surface and characterize each subcommand (and each behavior-changing
flag) along:

- **operation** — observe / create / mutate / destroy / execute / communicate / configure / authorize / control.
- **locus** — WHERE the effect lands, and the axis "read-only" hides: `local` (process → temp →
  worktree → home → machine → …) vs **`remote`** (a cloud API, another host, remote infra). A command
  that only "reads" but reaches a REMOTE system, or that changes out-of-workspace state (cloud/infra,
  a remote branch), can NEVER sit in an auto-approve level — SafeWrite is LOCAL-only. `terraform
  apply` is not above the line because it "isn't read-only"; it is `execute`/`configure` on a
  `remote` locus, mutating infrastructure irreversibly over the network.
- **network** — none / loopback / outbound-fetch / inbound-listen; and does it SEND host data (an exfil surface)?
- **execution** — does it run code, and whose? (self / caller-inline / caller-file / ambient-config /
  network-sourced — the last is the install / supply-chain surface, e.g. `npm install` postinstall.)
- **reversibility & persistence** — trivially undone / recoverable (VCS/snapshot) / effortful (backups
  only) / irreversible; and does it persist as data, RECONFIGURE future commands, or INSTALL
  executables/services/hooks?
- **secret & disclosure** — does it read a credential store, and where does the output flow (the
  model / a local file / a remote / the public)?

The `description` field IS that profile, written per subcommand as an independent risk assessment (as
if by a researcher who doesn't know how we'll use it): which subs observe vs mutate, which reach the
network or REMOTE infra, which execute provider/plugin code, which are irreversible, which touch
credentials. Call out subtleties — subs that delegate to an inner command, a flag that flips a sub
from one profile to another (a `-w`/`-i` write flag, an `--exec`), read vs write modes on one sub.
Record project velocity (release cadence, flag-surface stability) — look it up, don't guess. Do NOT
reference safe-chains internals (SafeWrite/SafeRead/Inert, handlers, allowlists) in the description —
describe the command.

### Then the classification picks the level

The `level` on a command/sub is the coarse projection of that behavioral profile onto the legacy
band (the engine's facet levels — `inert ⊂ read-local ⊂ write-local ⊂ developer` — are the finer
model this maps to):
- `Inert` — no real state observed or changed (`--version`, `--help`, arithmetic).
- `SafeRead` — observes LOCAL worktree state only.
- `SafeWrite` — creates/mutates LOCAL worktree data; no remote reach, no code install, no
  mass/irreversible destruction.
- Above the band — remote/infra effects, code execution or installation, irreversible or
  out-of-workspace destruction, credential extraction — is NOT auto-approved. Mark those subcommands
  `candidate = true` with the behavioral reason, so future contributors don't re-evaluate them (and
  users on older versions still get a "not found" no-op rather than a silent gap).

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
