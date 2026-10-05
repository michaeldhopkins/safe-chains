# Fuzzing: what is specific to safe-chains

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
