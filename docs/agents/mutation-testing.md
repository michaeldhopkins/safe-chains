# Mutation testing: what is specific to safe-chains

General method is in the `rust-mutation-testing` skill. This is what is specific to safe-chains.

- **Pre-release check (before each push to `main`, never after the push in CI):** `cargo mutants
  --in-diff` (or `scripts/mutants-run.sh --in-diff` under a wall-clock budget) runs on the
  release's changed code, from the last covered base (`scripts/mutants-base.sh <last-covered-base>`)
  to the release tree, before the push lands:
  ```bash
  base="$(scripts/mutants-base.sh "${LAST_COVERED_BASE:-origin/main}")"
  # Force a/ b/ prefixes. diff.noprefix drops them; cargo-mutants then matches
  # no files and exits 0. Pin diff.mnemonicPrefix the same way.
  git -c diff.mnemonicPrefix=false -c diff.noprefix=false diff "$base"..HEAD > mutants.diff
  if [ -s mutants.diff ] && ! grep -q '^diff --git a/' mutants.diff; then
    echo "::error::diff lacks a/ b/ prefixes — cargo-mutants would silently match nothing"
    head -3 mutants.diff
    exit 1
  fi
  scripts/mutants-run.sh --no-shuffle -vV --in-diff mutants.diff
  # or directly: cargo mutants --no-shuffle -vV --in-diff mutants.diff
  ```
  A MISSED mutant blocks the release until killed by a test or excluded with its reason. Nothing
  runs in CI after the push, and the old post-push workflow (`mutants.yml`) and rotating whole-tree
  `--shard` slices are retired (2026-11-23).
- **Test-bound, and the profile is the lever.** A mutant's rebuild is 2-5s locally; its test run is
  the full suite. The integration tests spawn the debug binary a few hundred times, and unoptimized
  each spawn spent ~0.4s parsing the registry TOML inside the dependencies. `.cargo/mutants.toml`
  selects the `mutants` profile (Cargo.toml), which optimizes dependencies only: suite 124s to 45s,
  rebuild cost unchanged. Restricting the test command was not an option: `cli_gate`, `path_policy`
  and the hook tests drive the real binary (`CARGO_BIN_EXE_safe-chains`), which is built from the
  mutated tree, so they catch mutants the lib tests do not. If the baseline test step slows down,
  find the slowest test with `cargo nextest run --cargo-profile mutants --features fuzz-gen`
  (`substitution_is_never_more_permissive_than_a_path_it_could_produce` in
  `src/substitution_property_tests.rs` classifies each abstraction once per root and suffix across
  threads after a serial pass grew the baseline to 149s).
- **The `fuzz-gen` feature is on** for mutation runs, or `gen-fuzz-corpus` is never built and all of
  its mutants read as MISSED. CI's test job runs its tests the same way.
- **Exclusion:** `gen-fuzz-corpus`'s `main` (binds the checkout path and prints counts; `generate`
  holds the logic and is tested). Reason in `.cargo/mutants.toml`.
- **Equivalent, not excluded:** `delete !` in `sed_cluster`'s `-f` arm (`consumes_next: !has`).
  `scan_sed` flags every `-f` spelling and `resolve_sed` worst-cases before the cluster parser runs,
  so the `ScriptFile` arm's word count is never observed.
- **Equivalent, not excluded:** the `"bash" | "sh" | "zsh" | "dash" | "ksh"` arm of
  `refuses_its_arguments` (`direct_script::runs_packaged_or_frozen`). The shells' executor already
  refuses a script under `node_modules` or a frozen folder through `frozen_reach`, so deleting the arm
  changes no verdict; `a_shell_handed_a_packaged_script_is_refused_in_every_spelling` pins the
  behaviour either way. Reported by the 0.242.0 release run (2026-11-11).
- **Findings of past runs** are in `docs/agents/mutation-findings.md`; add an entry there when you
  kill a pre-release run's misses.
- **Hermeticity it surfaced:** cargo-mutants builds in a copy under `$TMPDIR`, and
  `the_path_policy_corpus_holds` used the checkout itself as the workspace under the real `$HOME`, so
  its baseline failed there (a sibling is `adjacent` only under `$HOME`). The test now builds its
  workspace and peer under a `HOME` of its own.
- **Timeouts, budget wrapper, and loop-counter exclusions:** `.cargo/mutants.toml` sets
  `timeout_multiplier = 2.0` (a hung mutant costs 2x baseline rather than 5x; not lower, since a
  real mutant runs the whole suite and a spurious timeout reads as caught). `scripts/mutants-run.sh`
  runs `cargo mutants` under `MUTANTS_BUDGET` (default 900s), stops it with `TERM`, and decides the
  verdict from `mutants.out/missed.txt` rather than the exit status alone (exit 3 used to map to 0
  even with misses); `tests/mutants_run.rs` covers it with a stand-in `cargo`. In
  `Token::content_outside_double_quotes`, mutating `i += 1` to `-=`/`*=` stops the index while the
  loop pushes into a `Vec` without bound until OOM or timeout; those mutants can only ever time out,
  so `.cargo/mutants.toml` excludes them (10 of the function's 21; checked with `--list`), and the
  redundant `i + 1 < len` backslash guard was removed in favour of a trailing-backslash test. Any
  new loop whose counter can be mutated to stand still while allocating per iteration needs the same
  exclusion.
