# Mutation testing: what is specific to safe-chains

General method is in the `rust-mutation-testing` skill. This is what is specific to safe-chains.

- **Per change** (`.github/workflows/mutants.yml`, not gating): `--in-diff` on every PR and push to
  `main`, skipped with a warning above 7 selected mutants; and on each push to `main` one
  **rotating slice**, `--shard (run_number % 1024)/1024`. PR runs advance the same counter, so
  coverage is roughly one pass per 1024 runs rather than exactly one per 1024 pushes. There is no
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
- **Findings of past slices** are in `docs/agents/mutation-findings.md`; add an entry there when you kill a slice's misses.
- **Hermeticity it surfaced:** cargo-mutants builds in a copy under `$TMPDIR`, and
  `the_path_policy_corpus_holds` used the checkout itself as the workspace under the real `$HOME`, so
  its baseline failed there (a sibling is `adjacent` only under `$HOME`). The test now builds
  `~/projects/safe-chains` and a peer under a HOME of its own.

- **Every CI run cancelled at the 20-minute timeout, 2026-10-12 to 10-13** (runs 38266072545 to
  38423646672): the baseline test step had grown from 98s to 338s, because one exhaustive test,
  `substitution_is_never_more_permissive_than_a_path_it_could_produce`, took 143s of a 149s local
  suite. It classified each abstraction once per refused witness and ran serially; it now
  classifies each once per root and suffix, one thread per context, in
  `src/substitution_property_tests.rs` (suite 149s to 54s under the `mutants` profile). If the
  baseline test step passes ~150s on the runner again, find the slowest test with `cargo nextest
  run --cargo-profile mutants --features fuzz-gen` before touching N or the timeout.
- **N = 1024 and an in-diff cap of 7, from 2026-10-13.** With that test fixed, the runner's baseline
  was 100s build + 166s test and each mutant about 100s of wall clock at `-j2` (rebuilds of 40-90s,
  not the laptop's 2-5s), so a 13-mutant slice at N = 512 still hit the timeout. 1024 gives about 5
  mutants a slice.
- **Run 38612754676 (2026-10-14), cancelled at the timeout with nothing MISSED:** its slice held 7
  mutants in `Token::content_outside_double_quotes`. 6 were caught before the 20-minute limit. A
  local run of all 27 mutants in that function took 43 minutes: 18 caught, 6 timeouts (a `+=` turned
  into `-=` or `*=` never advances the loop, so each runs until the test timeout), and 3 MISSED. All
  three MISSED mutated the `i + 1 < len` guard on a backslash, which was redundant because the loop
  exits on overshoot either way. The guard is gone, and a trailing-backslash test pins what it did.
  The six timeouts are each about 280s of wall clock, so a slice that lands on several of them can
  run past the job limit however small N is.
- **Run 38628070688 (2026-10-14), cancelled again, the cause fixed rather than N.** A hung mutant
  runs to cargo-mutants' auto timeout, 5x the baseline test time (~280s here, over 800s on the
  runner's 166s baseline), so N cannot bound a slice that lands on several. Three changes:
  `timeout_multiplier = 2.0` in `.cargo/mutants.toml` (a hang now costs 2x the baseline: measured
  116s each against a 57s baseline, both timeouts of lines 110 and 124 of `Token::content_outside_double_quotes`;
  not lower, since a real mutant runs the whole suite and a spurious timeout reads as caught);
  `scripts/mutants-run.sh` runs cargo-mutants under a 15-minute budget (`MUTANTS_BUDGET`, 15 minutes), stops it
  with TERM and reports the slice as incomplete instead of letting the 20-minute job limit cancel
  the run; and the verdict now reads `mutants.out/missed.txt`, not the exit status (exit 3 used to
  map to 0 even with misses). `tests/mutants_run.rs` covers it with a stand-in cargo. Both jobs
  call the script. If hangs still dominate, a per-test cap under nextest is the next lever.
