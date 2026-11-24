# Fuzzing: what is specific to safe-chains

General fuzzing method — the two budgets, the seeding ladder, coverage, CI shape, the gotchas — is
in the `rust-fuzzing` skill. This section is only what is specific to safe-chains.

- **Targets** (`fuzz/fuzz_targets/`; each file's header states its properties in full):
  - `parse` — `is_safe_command` on arbitrary bytes: never panics, never hangs, across parse → CST →
    engine → handlers. It **discards the verdict**, so it finds availability bugs only.
  - `equivalence` — a semantics-preserving respelling (flag forms, env-var forms, a value spelled
    byte by byte in `$'\xHH'`) cannot change the verdict, and a word whose value is unknown (`$X`,
    `$(…)`, a glob) is never approved where a flag it could be is refused.
  - `hook_envelope` — the `targets/*` hook I/O never panics and never emits a grant it was not
    asked for.
  - `explain_render` — the explanation describes the verdict that was enforced, and a command
    cannot forge a marker line or smuggle a control/bidi character into it.
  - `suggest_roundtrip` — every config `--suggest` generates parses back, and merging never drops
    what was there.
  - `level_monotonic` — a stricter level never approves what a looser one refused; with the folder
    unknown, no folder level approves what the project root refuses, and `reads` ⊆ `developer` ⊆
    `workspace`. Its `seed-*` inputs are the unknown-folder regressions.
  - `config_load` — a repo `.safe-chains.toml` (the one attacker-placed input) never aborts the
    loader, loads NOTHING when it is not valid TOML, and loads deterministically.
  - `setup_merge` — `--setup` into an arbitrary existing settings file: on refusal the file is
    byte-identical, on success it still parses.
  - `path_admit` — the credential shield outranks every package-content read admit, and those
    admits never grant a write.
  - `gate_prefilter` — a declared path gate never skips a value its own judge would refuse.
  - `config_edit` — edits keep a file loading; grant-then-remove is the identity.
  - `mcp_session` — one reply per request id.
- **Not fuzzed:** `docs.rs` (no security property) and `pathctx` (proptests cover it and shrink
  better for a pure function).
- **CI and the pre-release check:** `fuzz-replay.yml` replays every target's committed
  `fuzz/corpus/<target>/seed-*` inputs from the checkout (`-runs=0`) on every push and PR — the CI
  regression gate (skipping docs-only pushes, `**.md` and `docs/**`, since nothing fuzzed reads
  either). Discovery bursts run before each push to `main` in the pre-release check on the targets
  the diff touches, never in CI after the push or on a schedule:
  ```bash
  # when `parse` or commands/**/*.toml changed, refresh the registry seeds and dictionary first:
  cargo run --bin gen-fuzz-corpus --features fuzz-gen
  # for each target <target> the diff touches:
  cargo +nightly fuzz build <target>
  fuzz/burst.sh fuzz/target/<triple>/release/<target> <target> <seconds>
  ```
  `tests/fuzz_targets_wired.rs` keeps `fuzz-replay.yml`'s target matrix equal to `fuzz/Cargo.toml`
  and refuses a `schedule:` trigger or a post-push burst workflow.
- **`fuzz/burst.sh` runs a timed burst and merge locally** (`fuzz/burst.sh <binary> <target> <seconds>`, from the
  repo root): mutation timed from when the corpus has loaded (libFuzzer's `-max_total_time` counts
  the load, which on a large corpus can exceed the whole budget), then `-merge=1` into the corpus.
  Finds go to `fuzz/new/<target>`, so bursting targets one after another never merges
  one target's finds into the next; the committed `seed-*` inputs keep their names through the
  merge; `fuzz/dict/<target>.dict` is used when present. A timeout reported after its SIGINT is
  libFuzzer stalling on the way out (2026-10-05, `suggest_roundtrip`): the unit is replayed alone
  and dropped if it finishes. `tests/fuzz_burst.rs` holds all of this against a stand-in libFuzzer.
- **No nightly or post-push CI burst.** The nightly (retired 2026-09-26) ran 05:00 UTC: three 5h
  shards on `parse` plus 1h on each property target. Every real find came in a target's first days;
  after the first week of August it ran seven weeks without another, and its later red nights were
  job timeouts (`config_load` overrunning `timeout-minutes`), not findings. The post-push CI burst
  was retired on 2026-11-23 in favour of the pre-release check, which fuzzes changed targets before
  the push lands so fixes ship in the same release.
- **Seeding is registry-derived.** `src/bin/gen_fuzz_corpus.rs` (feature `fuzz-gen`,
  `cargo run --bin gen-fuzz-corpus --features fuzz-gen`) reads `commands/**/*.toml` and emits the
  `examples_safe`/`examples_denied` invocations as seeds and the command/subcommand/flag vocabulary
  as a `-dict` for `parse`. New commands are covered automatically — **no hand-maintained fuzz
  corpus.** Regenerate them before a `parse` burst so `fuzz/burst.sh` merges them in.
  Generated `gen-*` seeds and `fuzz/dict/` are git-ignored. The property targets take other input
  domains (a flag value, an envelope, a config file), so the registry seeds do not apply to them.
- **Regression seeds are the one hand-kept input.** Starting seeds and minimized finds are committed
  as `seed-*` in each target's corpus (`.gitignore` lets `seed-*` through under
  `fuzz/corpus/<target>/` for every target), so the replay exercises every target and holds past
  findings down. `hook_envelope/seed-codex-permission-request` is a Codex
  `PermissionRequest` envelope (not a find) so the second Codex format is reached.
  `explain_render/seed-unclosed-brace-nest` (2026-09-26): an unclosed `{` nest parsed once as a
  brace group and again as `{`, doubling per level; `cst/budget.rs` and `cst/parse.rs` count work
  instead of timing it. `level_monotonic/seed-timeout-brace-glob` (2026-10-08): brace copies ending
  `/bunx` made each delegation re-parse every remaining copy (9.6 MB from 769 B);
  `cst/classify_budget.rs` caps parser bytes too. `gate_prefilter/seed-timeout-repeated-stars`
  (2026-10-17): `--output-dir=/**/******./..` made `pathctx::physical::resolve` match `******\.`
  across `/`, where recursive `*` backtracking took $\binom{N+6}{6}$ calls per filename;
  `glob::matches` now matches in linear time and `physical::resolve` charges entries walk-wide.
- **Measured coverage** (region, authored source): mutation corpus alone ~26%, registry seeds alone
  ~37%, **combined ~61%**. Seeds unlock per-command grammars; mutation covers parser byte-paths.
- **Two kinds of coverage hole, and what each means:**
  - **Un-exampled commands** — a handler at ~0% (`glab`, `magick`, `sysctl`) almost always has NO
    `examples_safe`/`examples_denied`, so nothing seeds it and byte mutation never synthesizes
    `glab mr list`. Adding examples is the **cheapest coverage win** and doubles as a
    `toml_examples_match_dispatch` guard.
  - **Out-of-scope layers** — `targets/*`, `docs.rs`/`registry/docs.rs`, and `suggest.rs` sit at ~0%
    in `parse` because `is_safe_command` never calls them; reaching them takes their own target
    (`hook_envelope`, `suggest_roundtrip`), not seeding `parse`.
  - **`cst/explain.rs` + `cst/display.rs` in property tests:** `arbitrary_command_strings_never_panic`
    and `classifier_terminates_on_adversarial_input` in `handler_property_tests.rs` also run
    `explain(&line).render()` alongside `command_verdict`, holding the renderer (`--explain` and
    hook injected context, where a panic fails OPEN) to the same contract.
