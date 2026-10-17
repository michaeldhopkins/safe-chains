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
  merge; `fuzz/dict/<target>.dict` is used when present. A timeout reported after its SIGINT is
  libFuzzer stalling on the way out (2026-10-05, `suggest_roundtrip`): the unit is replayed alone
  and dropped if it finishes. `tests/fuzz_burst.rs` holds all of this against a stand-in libFuzzer.
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
  its target's corpus (`.gitignore` lets `seed-*` through for `parse`, `explain_render`,
  `hook_envelope`, `level_monotonic`, `gate_prefilter`, `config_edit` and `mcp_session`), so the
  replay holds it down. `hook_envelope/seed-codex-permission-request` is a Codex
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
