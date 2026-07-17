# Fuzzing safe-chains

Coverage-guided fuzzing of the command classifier with [`cargo-fuzz`] / libFuzzer. This is the
runtime extension of the `arbitrary_command_strings_never_panic` proptest: the invariant is that
`is_safe_command` (and everything under it — shell split, CST, resolvers) **never panics or hangs**
on any input.

This directory is a **standalone workspace** (note the empty `[workspace]` in `Cargo.toml`), so the
root `cargo build` / `test` / `clippy` / `deny` never see it. It builds only under `cargo fuzz`
(cargo-fuzz needs nightly for `-Zsanitizer`); the main crate stays on stable.

## One-time setup

```sh
rustup toolchain install nightly --component rust-src
cargo install cargo-fuzz
```

## Run

Always select nightly with `+nightly` — cargo-fuzz invokes `cargo build --manifest-path fuzz/…`
from the **repo root**, and rustup resolves a `rust-toolchain.toml` by the *current directory*, so a
toolchain file inside `fuzz/` is NOT picked up. The `+nightly` override propagates (via
`RUSTUP_TOOLCHAIN`) to the inner build:

```sh
# from the repo root
cargo +nightly fuzz build parse                      # smoke-test: compiles + links?
cargo +nightly fuzz run parse                        # runs until a crash or Ctrl-C
cargo +nightly fuzz run parse -- -max_total_time=28800   # 8-hour overnight budget
```

The corpus lives in `fuzz/corpus/parse/`; the `seed-*` files are committed starting inputs, and
libFuzzer's hash-named additions are gitignored. Shrink an overgrown corpus with
`cargo fuzz cmin parse`.

## A crash

libFuzzer writes the reproducing input to `fuzz/artifacts/parse/crash-<hash>`. Reproduce and debug:

```sh
cargo fuzz run parse fuzz/artifacts/parse/crash-<hash>
cargo fuzz fmt parse fuzz/artifacts/parse/crash-<hash>   # show the input as a string
```

Then add the minimized input as a regression case to the `no-panic` proptest/corpus and fix the bug.

## CI

`.github/workflows/fuzz.yml` runs `parse` nightly on a Linux runner (ASan is happiest on Linux),
caching the corpus between runs so coverage accumulates. Trigger a manual run from the Actions tab
(`workflow_dispatch`) with a custom duration.

## Reproducibility

`+nightly` uses whatever nightly is installed. For the overnight job, pin a **dated** nightly so a
churny nightly can't break the build mid-run: `rustup toolchain install nightly-2026-07-10` then run
`cargo +nightly-2026-07-10 fuzz run parse …` (and set the CI's `dtolnay/rust-toolchain` to the same
date). Bump it deliberately.

[`cargo-fuzz`]: https://github.com/rust-fuzz/cargo-fuzz
