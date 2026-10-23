# Mutation testing: findings of past slices

What each rotating slice found and which tests killed it. Add an entry when you kill a slice's misses. The job, sizing and costs are in `mutation-testing.md`.

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
- **CI slice of run 37348486489 (2026-10-05):** 3 MISSED of 10 viable: `print_docs` emptied,
  `--explain`'s exit code inverted, and `||` to `&&` in `canonicalize_home`'s no-home guard. Killed
  by `list_commands_prints_the_command_reference` and `explain_exits_with_the_verdict` in
  `tests/cli_gate.rs`, and the property `without_a_usable_home_every_word_is_left_alone`.
- **CI slice of run 37362532620:** 2 MISSED of 9 viable, both in `src/allowlist.rs`: the bare-`~`
  guard in `canonicalize_home` set to `false`, and `Matcher::is_empty` returning `true` (only ever
  asserted on empty matchers). Killed by `a_usable_home_expands_both_tilde_spellings` and a
  non-empty assertion in `parse_exact_pattern` and `parse_legacy_colon_star`. The next slice
  (run 37365593962) missed `-` to `+` in `glob_matches`; a local run over the whole file then found
  two more in its middle-piece loop. `a_glob_match_holds_every_piece` and
  `a_glob_prefix_and_suffix_may_not_overlap` kill all three, and the file now has no survivors.
- **CI slice of run 37505805989 (2026-10-06):** 3 MISSED of 9 viable: `normalize_redirs` emptied
  (the roundtrip property renders the normalized form, so a dropped redirect round-trips cleanly),
  and `*` to `+` in either factor of `ROTATE_AT_BYTES`. Killed by the property
  `normalizing_keeps_every_redirect` in `src/cst/normalize_tests.rs` and
  `the_rotation_cap_is_sixteen_mebibytes` in `src/decisionlog/tests.rs`. The next slice (run
  37508602217) missed `Outcome`'s `Display` returning nothing; `an_outcome_displays_as_its_log_name`
  kills it. Run 37511064101 missed `rotate_if_large` emptied and `<` to `<=` at the rotation cap;
  `appending_to_a_full_log_starts_a_fresh_one` (a sparse file at the real cap) and
  `a_file_exactly_at_the_cap_rotates_and_one_byte_under_does_not` kill them. Run 37511608360
  missed `unix_millis` returning 0 or 1; `the_clock_reads_the_current_time_in_milliseconds` kills
  both. Run 37515672271 missed `-` to `+` or `/` in `civil_from_days`'s negative-era branch;
  `civil_from_days_handles_dates_before_year_zero` kills both. Five slices in a row landed in `src/decisionlog.rs`: a whole-file local run over it
  (128 mutants) is owed, in `TODO.md`.
- **CI slice 113/1024 (run 39045369068, 2026-10-17):** 1 MISSED (`delete !` on `!flag.starts_with("--")` in
  `output_flag_value`, `src/registry/mod.rs:409:52`), and stopped at the 15-minute budget during `delete !` on
  `!r.is_empty()` (`410:84`) with `flag_present` (`423`–`426`) still queued. `data_export_gates_its_output_file`
  asserted only `Verdict::Denied` on `supabase db dump -f/etc/passwd`, which still held when `output_flag_value`
  dropped the glued short path because `presents_unlisted_flag` also marks `-f/etc/passwd` `unclassified`. Now killed
  by asserting the `SystemIntegrity` `Create` capability in `data_export_gates_its_output_file` (`src/engine/resolve.rs`)
  and by `output_flag_value_extracts_every_spelling_and_rejects_non_short_glued_forms` and
  `flag_present_matches_bare_glued_and_short_clusters_only` in `src/registry/arity_tests.rs`.
- **Pre-release runs (2026-10-23):** the first run over the stack (763 mutants) missed 8 and the
  second, over the link-following gate fix (40), missed 6, all killed by tests named in their
  commits. Three were equivalent and went by refactor; one, `&&` to `||` in `gated_policy`, is
  excluded in `.cargo/mutants.toml` with its reason. Traps met on the way:
  - A prover that restores a file without touching it leaves cargo's stale build in place, so the
    next mutant reports KILLED for the previous one's binary. Touch the file after restoring it,
    and count a KILLED only when the unmutated baseline passed in the same run.
  - Given a custom `--output`, the run script read the wrong `missed.txt` and passed a run with
    21 MISSED; it now pins `--output .`.
  - `functions.rs:88` (`||` to `&&`) was a TIMEOUT on a slow runner and MISSED on a fast one: a
    timeout that ends a recursion cap is not always a catch, so give such a guard a test that
    shows both sides of the cap.
  - Setting the build directory under `/tmp` moves `CARGO_TARGET_TMPDIR` there too, and the
    tests that build scratch projects in it then read them as temporary folders and fail.
