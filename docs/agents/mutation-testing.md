# Mutation testing: what is specific to safe-chains

General method is in the `rust-mutation-testing` skill. This is what is specific to safe-chains.

- **Per change** (`.github/workflows/mutants.yml`, not gating): `--in-diff` on every PR and push to
  `main`, skipped with a warning above 20 selected mutants; and on each push to `main` one
  **rotating slice**, `--shard (run_number % 512)/512`. PR runs advance the same counter, so
  coverage is roughly one pass per 512 runs rather than exactly one per 512 pushes. There is no
  whole-tree sweep: ~5,300 mutants at ~39s of runner wall clock each is ~58 hours.
- **N = 512, from the runner.** The first CI slice (run 36363446362, slice 1/256, 21 mutants) took
  16.7 minutes: baseline 105s build + 98s test, then ~39s of wall clock per mutant at `-j2`, against
  a 20-minute job timeout. The `mutants` profile did apply (`--profile=mutants` in its baseline
  log); the runner is just slower than the laptop, where slice 128/256 took 6m45s (21s build + 45s
  test). At 512 a slice is ~10 mutants, ~10 minutes. If slices approach the timeout again, raise N
  rather than the timeout. The in-diff cap of 20 is from the same runner rate.
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
- **CI slice 1/256 (run 36363446362):** 1 MISSED of 17 viable: `!=` to `==` in `facet_breakdown`'s
  one-segment guard. Nothing called the function directly. Now killed by
  `facet_breakdown_profiles_a_single_command_and_declines_a_chain` in `tests/facet_breakdown.rs`. Writing it
  showed that a PIPELINE is one segment to `cst::explain`, so `facet_breakdown("cat x | rm -rf /")`
  still gets the flat split the guard exists to prevent (a diagnostic only, not a verdict; see
  TODO.md).
- **Hermeticity it surfaced:** cargo-mutants builds in a copy under `$TMPDIR`, and
  `the_path_policy_corpus_holds` used the checkout itself as the workspace under the real `$HOME`, so
  its baseline failed there (a sibling is `adjacent` only under `$HOME`). The test now builds
  `~/projects/safe-chains` and a peer under a HOME of its own.

