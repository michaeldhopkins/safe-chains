# Rust

### `atuin`
<p class="cmd-url"><a href="https://atuin.sh/">https://atuin.sh/</a></p>

- **default-config**: Flags: --help, -h
- **doctor**: Flags: --help, -h
- **help**: Positional args accepted
- **info**: Flags: --help, -h
- **init**: Flags: --disable-ai, --disable-ctrl-r, --disable-up-arrow, --help, -h. Positional args accepted
- **search**: Flags: --cmd-only, --exit, --exit, --filter-mode, --help, --human, --inline-height, --interactive, --keymap-mode, --reverse, --shell-up-key-binding, --cwd, --exclude-cwd, --exclude-exit, --exit, --limit, -h, -i, -r. Valued: --after, --before, --cmd-only, --cwd, --delete, --delete-it-all, --exit, --filter-mode, --format, --limit, --offset, --search-mode, --session, --user, -c, -e, -f. Positional args accepted
- **stats**: Flags: --help, -h. Valued: --count, --ngram. Positional args accepted
- **uuid**: Flags: --help, -h
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h, -V

**Examples:**

- `atuin doctor`
- `atuin default-config`
- `atuin uuid`
- `atuin info`
- `atuin stats`
- `atuin search foo`
- `eval "$(atuin init bash)"`
- `eval "$(atuin init zsh --disable-ctrl-r --disable-up-arrow)"`
- `eval "$(atuin init fish --disable-ai)"`

### `bacon`
<p class="cmd-url"><a href="https://github.com/Canop/bacon">https://github.com/Canop/bacon</a></p>

- Allowed standalone flags: --help, --prefs, --version, -h, -v
- Bare invocation allowed

### `cargo`
<p class="cmd-url"><a href="https://doc.rust-lang.org/cargo/commands/">https://doc.rust-lang.org/cargo/commands/</a></p>

- **about generate**: Flags: --all-features, --fail, --frozen, --help, --locked, --no-default-features, --offline, --workspace, -V, -h. Valued: --color, --features, --format, --manifest-path, --name, --target, --threshold, -L, -c, -m, -n
- **about**: Flags: --help, --version, -V, -h. Valued: --color, -L
- **asm**: Flags: --all-features, --bin, --debug-info, --example, --features, --frozen, --full-name, --help, --lib, --locked, --no-color, --no-default-features, --offline, --release, --rust, --simplify, --source, --test, --verbose, --version, -V, -h. Valued: --asm-style, --bench, --build-type, --manifest-path, --package, --target, --target-cpu, --target-dir, --unstable, -p
- **audit**: Flags: --deny, --help, --json, --no-fetch, --stale, -h, -n, --quiet, -q, -v. Valued: --color, --db, --file, --ignore, --target-arch, --target-os, -f
- **bench**: Flags: --all, --workspace, --all-features, --all-targets, --benches, --bins, --doc, --examples, --frozen, --future-incompat-report, --help, --ignore-rust-version, --keep-going, --lib, --locked, --no-default-features, --no-fail-fast, --no-run, --offline, --release, --tests, --timings, --unit-graph, -h, --quiet, -q, -v. Valued: --bench, --bin, --color, --example, --features, --jobs, --manifest-path, --message-format, --package, --profile, --target, --target-dir, --test, -Z, -j, -p
- **bloat**: Flags: --crates, --filter, --help, --lib, --no-default-features, --release, --time, --wide, -h. Valued: --bin, --example, --features, --jobs, --manifest-path, --message-format, --package, --target, -j, -n, -p
- **build**: Flags: --all, --workspace, --all-features, --all-targets, --benches, --bins, --build-plan, --examples, --frozen, --future-incompat-report, --help, --ignore-rust-version, --keep-going, --lib, --locked, --no-default-features, --offline, --release, --tests, --timings, --unit-graph, -h, --quiet, -q, -v. Valued: --bench, --bin, --color, --example, --features, --jobs, --manifest-path, --message-format, --package, --profile, --target, --target-dir, --test, -Z, -j, -p
- **check**: Flags: --all, --all-features, --all-targets, --benches, --bins, --examples, --frozen, --future-incompat-report, --help, --ignore-rust-version, --keep-going, --lib, --locked, --no-default-features, --offline, --release, --tests, --timings, --unit-graph, --version, --workspace, -V, -h, --quiet, -q, -v. Valued: --bench, --bin, --color, --example, --exclude, --features, --jobs, --manifest-path, --message-format, --package, --profile, --target, --target-dir, --test, -Z, -j, -p
- **clean**: Flags: --doc, --dry-run, --frozen, --help, --locked, --offline, --release, --workspace, -h, -n, --quiet, -q, -r, -v. Valued: --color, --lockfile-path, --manifest-path, --package, --profile, --target, --target-dir, -Z, -p
- **clippy**: Flags: --all, --all-features, --all-targets, --benches, --bins, --examples, --frozen, --future-incompat-report, --help, --ignore-rust-version, --keep-going, --lib, --locked, --no-default-features, --no-deps, --offline, --release, --tests, --timings, --unit-graph, --version, --workspace, -V, -h, --quiet, -q, -v. Valued: --bench, --bin, --color, --example, --exclude, --features, --jobs, --manifest-path, --message-format, --package, --profile, --target, --target-dir, --test, -Z, -j, -p
- **config get**: Flags: --frozen, --help, --locked, --offline, --show-origin, -h, --quiet, -q, -v. Valued: --color, --format, --merged, -Z
- **criterion**: Flags: --all, --all-features, --all-targets, --benches, --bins, --debug, --examples, --frozen, --help, --lib, --locked, --no-default-features, --no-fail-fast, --no-run, --offline, --tests, --verbose, --version, --workspace, -V, -h, -v. Valued: --bench, --bin, --color, --criterion-manifest-path, --example, --exclude, --features, --history-description, --history-id, --jobs, --manifest-path, --message-format, --output-format, --package, --plotting-backend, --target, --target-dir, --test, -Z, -j, -p
- **cyclonedx**: Flags: --all, --all-features, --help, --license-strict, --no-default-features, --quiet, --target-in-filename, --top-level, --verbose, --version, -V, -a, -h, -q, -v. Valued: --describe, --features, --format, --license-accept-named, --manifest-path, --override-filename, --spec-version, --target, -F, -f
- **deny**: Flags: --all-features, --help, --no-default-features, -h, --quiet, -q, -v. Valued: --color, --exclude, --features, --format, --manifest-path, --target, --workspace
- **doc**: Flags: --all, --all-features, --bins, --document-private-items, --examples, --frozen, --future-incompat-report, --help, --ignore-rust-version, --keep-going, --locked, --no-default-features, --no-deps, --offline, --open, --release, --timings, --unit-graph, --workspace, -h, --quiet, -q, -v. Valued: --bin, --color, --example, --features, --jobs, --manifest-path, --message-format, --package, --profile, --target, --target-dir, -Z, -j, -p
- **expand**: Flags: --all-features, --help, --lib, --no-default-features, --release, --tests, --ugly, -h. Valued: --bin, --color, --example, --features, --manifest-path, --package, --target, --theme, -p
- **fetch**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --lockfile-path, --manifest-path, --target, -Z
- **fmt**: Flags: --all, --check, --help, -h, --quiet, -q, -v. Valued: --manifest-path, --message-format, --package, -p
- **geiger**: Flags: --all, --all-dependencies, --all-features, --all-targets, --build-dependencies, --dev-dependencies, --forbid-only, --frozen, --help, --include-tests, --invert, --locked, --no-default-features, --no-indent, --offline, --prefix-depth, --quiet, --verbose, --version, -V, -a, -f, -h, -i, -q, -v. Valued: --color, --features, --format, --manifest-path, --output-format, --package, --section-name, --target, -Z, -p
- **generate-lockfile**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --manifest-path
- **help**: Positional args accepted
- **info**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --index, --registry
- **init**: Flags: --bin, --frozen, --help, --lib, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --edition, --name, --registry, --vcs, -Z
- **install**: Flags: --all-features, --bins, --debug, --force, --frozen, --help, --ignore-rust-version, --keep-going, --locked, --no-default-features, --no-track, --offline, --quiet, --timings, --verbose, -f, -h, -q, -v. Valued: --bin, --color, --example, --features, --jobs, --message-format, --path, --profile, --root, --target, --target-dir, -F, -Z, -j
- **license**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --manifest-path
- **llvm-cov**: Flags: --all-features, --all-targets, --help, --html, --json, --lcov, --lib, --locked, --no-cfg-coverage, --no-default-features, --no-fail-fast, --no-run, --open, --release, --text, -h. Valued: --bin, --branch, --codecov, --cobertura, --color, --example, --exclude, --features, --ignore-filename-regex, --ignore-run-fail, --jobs, --manifest-path, --output-dir, --output-path, --package, --profile, --target, --target-dir, --test, -j, -p
- **locate-project**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --manifest-path
- **machete**: Flags: --help, --skip-target-dir, --with-metadata, -V, -h
- **metadata**: Flags: --all-features, --frozen, --help, --locked, --no-default-features, --no-deps, --offline, -h, --quiet, -q, -v. Valued: --color, --features, --filter-platform, --format-version, --manifest-path
- **modules**: Flags: --all-features, --cfg-test, --help, --no-default-features, --no-externs, --no-fns, --no-modules, --no-sysroot, --no-traits, --no-types, --no-uses, --orphans, --sort-by-name, --sort-by-visibility, --types, --uses, --verbose, -h. Valued: --bin, --example, --features, --lib, --manifest-path, --package, --target, -p
- **msrv find**: Flags: --all-features, --bisect, --help, --ignore-lockfile, --include-all-patch-releases, --linear, --no-check-feedback, --no-default-features, --no-log, --no-user-output, -h. Valued: --component, --features, --log-level, --log-target, --manifest-path, --max, --maximum, --min, --minimum, --output-format, --path, --release-source, --target
- **msrv list**: Flags: --help, --no-log, --no-user-output, -h. Valued: --log-level, --log-target, --manifest-path, --output-format, --path, --variant
- **msrv show**: Flags: --help, --no-log, --no-user-output, -h. Valued: --log-level, --log-target, --manifest-path, --output-format, --path
- **msrv verify**: Flags: --all-features, --help, --ignore-lockfile, --include-all-patch-releases, --no-check-feedback, --no-default-features, --no-log, --no-user-output, -h. Valued: --component, --features, --log-level, --log-target, --manifest-path, --max, --maximum, --min, --minimum, --output-format, --path, --release-source, --rust-version, --target
- **msrv**: Flags: --help, --no-log, --no-user-output, --version, -V, -h. Valued: --log-level, --log-target, --manifest-path, --output-format, --path
- **new**: Flags: --bin, --frozen, --help, --lib, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --edition, --name, --registry, --vcs, -Z
- **nextest archive**: Flags: --all-features, --help, --locked, --no-default-features, --release, -h. Valued: --archive-file, --archive-format, --cargo-profile, --features, --manifest-path, --package, --target, --target-dir, -p
- **nextest list**: Flags: --all-features, --help, --lib, --locked, --no-default-features, --release, -T, -h. Valued: --bin, --color, --exclude, --features, --manifest-path, --message-format, --package, --partition, --profile, --target, --target-dir, --test, -E, -p
- **nextest run**: Flags: --all-features, --all-targets, --help, --lib, --locked, --no-capture, --no-default-features, --no-fail-fast, --release, --status-level, -h. Valued: --bin, --cargo-profile, --color, --exclude, --features, --jobs, --manifest-path, --package, --partition, --profile, --retries, --target, --target-dir, --test, --test-threads, --threads, -E, -j, -p
- **nextest show-config**: Flags: --help, -h
- **nextest**: Flags: --help, --version, -V, -h
- **outdated**: Flags: --aggressive, --color, --depth, --exit-code, --features, --help, --manifest-path, --packages, --root-deps-only, --verbose, --workspace, -R, -V, -d, -h, -n, --quiet, -q, -r, -v, -w. Valued: --color, --depth, --exclude, --features, --ignore, --manifest-path, --packages, -d, -e, -i, -p
- **package** (requires --list, -l): Flags: --all-features, --allow-dirty, --exclude-lockfile, --frozen, --help, --keep-going, --list, --locked, --no-default-features, --no-metadata, --no-verify, --offline, --workspace, -h, -l, --quiet, -q, -v. Valued: --color, --exclude, --features, --index, --jobs, --lockfile-path, --manifest-path, --message-format, --package, --registry, --target, --target-dir, -F, -Z, -j, -p
- **pkgid**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --manifest-path
- **public-api**: Flags: --all-features, --help, --no-default-features, --simplified, --version, -V, -h, -s. Valued: --color, --features, --manifest-path, --omit, --package, --target, -F, -p
- **publish** (requires --dry-run, -n): Flags: --all-features, --allow-dirty, --dry-run, --frozen, --help, --keep-going, --locked, --no-default-features, --no-verify, --offline, --workspace, -h, -n, --quiet, -q, -v. Valued: --color, --exclude, --features, --index, --jobs, --lockfile-path, --manifest-path, --package, --registry, --target, --target-dir, -F, -Z, -j, -p
- **read-manifest**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --manifest-path
- **report future-incompatibilities**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --id, --package, -Z, -p
- **run**: Runs this project's own code. Flags: --all-features, --frozen, --help, --ignore-rust-version, --locked, --no-default-features, --offline, --release, --timings, --unit-graph, -h, --quiet, -q, -v. Valued: --bin, --color, --example, --features, --jobs, --manifest-path, --message-format, --package, --profile, --target, --target-dir, -Z, -j, -p
- **search**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --index, --limit, --registry
- **semver-checks check-release**: Flags: --all-features, --default-features, --help, --verbose, -h, -v. Valued: --baseline-rev, --baseline-root, --baseline-version, --color, --config-path, --current-rustdoc, --exclude, --features, --manifest-path, --package, --release-type, --target, -j, -p
- **swift init**: Flags: --accept-all, --help, --macro, --plain, --silent, -h, -s, -y. Valued: --lib-type, --vcs
- **swift package**: Flags: --accept-all, --all-features, --help, --no-default-features, --release, --silent, --skip-toolchains-check, --suppress-warnings, -h, -r, -s, -y. Valued: --features, --lib-type, --name, --platforms, -F, -n, -p
- **swift**: Flags: --accept-all, --help, --silent, --version, -V, -h, -s, -y
- **test**: Flags: --all, --all-features, --all-targets, --benches, --bins, --doc, --examples, --frozen, --future-incompat-report, --help, --ignore-rust-version, --keep-going, --lib, --locked, --no-default-features, --no-fail-fast, --no-run, --offline, --release, --tests, --timings, --unit-graph, --workspace, -h, --quiet, -q, -v. Valued: --bench, --bin, --color, --example, --exclude, --features, --jobs, --manifest-path, --message-format, --package, --profile, --target, --target-dir, --test, -Z, -j, -p
- **tree**: Flags: --all-features, --duplicates, --frozen, --help, --ignore-rust-version, --locked, --no-dedupe, --no-default-features, --offline, -d, -e, -h, -i, --quiet, -q, -v. Valued: --charset, --color, --depth, --edges, --features, --format, --invert, --manifest-path, --package, --prefix, --prune, --target, -p
- **udeps**: Flags: --all, --all-features, --all-targets, --benches, --bins, --examples, --frozen, --help, --keep-going, --lib, --locked, --no-default-features, --offline, --release, --tests, --workspace, --version, -V, -h, --quiet, -q, -v. Valued: --backend, --bench, --bin, --color, --example, --exclude, --features, --jobs, --manifest-path, --message-format, --output, --package, --profile, --target, --target-dir, --test, -j, -p
- **update**: Flags: --aggressive, --dry-run, --frozen, --help, --locked, --offline, --recursive, --workspace, -h, --quiet, -q, -v. Valued: --color, --manifest-path, --package, --precise, -p
- **vendor**: Flags: --frozen, --help, --locked, --no-delete, --offline, --respect-source-config, --versioned-dirs, -h, --quiet, -q, -v. Valued: --color, --lockfile-path, --manifest-path, --sync, -Z, -s
- **verify-project**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --manifest-path
- **version**: Flags: --help, -h. Positional args accepted
- **vet check**: Flags: --help, -h
- **vet dump-graph**: Flags: --help, -h. Valued: --depth
- **vet explain-audit**: Flags: --help, -h. Valued: --criteria
- **vet suggest**: Flags: --help, -h
- **vet**: Flags: --frozen, --help, --locked, --no-all-features, --no-default-features, --no-minimize-exemptions, --no-registry-suggestions, --version, -V, -h. Valued: --cache-dir, --cargo-arg, --features, --filter-graph, --manifest-path, --output-format, --store-path, --verbose
- Allowed standalone flags: --help, --version, -V, -h

**Examples:**

- `cargo run`
- `cargo run --release`
- `cargo run --bin server`
- `cargo build`
- `cargo test`
- `cargo fmt`
- `cargo fmt --all`
- `cargo fmt --check`
- `cargo fmt --manifest-path ./crates/core/Cargo.toml`
- `cargo about --version`
- `cargo about --help`
- `cargo about generate about.hbs`
- `cargo swift --version`
- `cargo swift package --help`
- `cargo swift init KeelFFI`
- `cargo swift init KeelFFI --lib-type static --vcs git`
- `cargo swift package --platforms ios --name KeelFFI --release --accept-all`
- `cargo swift package -p ios -p macos --name Foo`
- `cargo swift package --features foo --all-features`

### `diesel`
<p class="cmd-url"><a href="https://diesel.rs/guides/getting-started">https://diesel.rs/guides/getting-started</a></p>

- **completions**: Flags: --help, -h. Positional args accepted
- **help**: Positional args accepted
- **migration generate**: Flags: --help, --no-down, --no-up, -h. Valued: --diff-schema, --migrations-dir, --sql, --version
- **migration list**: Flags: --help, -h. Valued: --migrations-dir
- **migration pending**: Flags: --help, -h. Valued: --migrations-dir
- **print-schema**: Flags: --help, --with-docs, --with-docs-config, -h. Valued: --except-tables, --filter, --import-types, --only-tables, --patch-file, --schema, -s
- Allowed standalone flags: --help, --version, -h, -V

### `rustup`
<p class="cmd-url"><a href="https://rust-lang.github.io/rustup/">https://rust-lang.github.io/rustup/</a></p>

- **component list**: Flags: --help, --installed, -h, -v. Valued: --toolchain
- **doc**: Flags: --alloc, --book, --cargo, --core, --edition-guide, --embedded-book, --help, --nomicon, --path, --proc_macro, --reference, --rust-by-example, --rustc, --rustdoc, --std, --test, --unstable-book, -h. Valued: --toolchain
- **run**: delegates to inner command
- **show**: Flags: --help, --installed, -h, -v
- **target add**: Flags: --help, -h. Valued: --toolchain
- **target list**: Flags: --help, --installed, -h, -v. Valued: --toolchain
- **target remove**: Flags: --help, -h. Valued: --toolchain
- **toolchain list**: Flags: --help, --installed, -h, -v. Valued: --toolchain
- **which**: Flags: --help, -h. Valued: --toolchain
- Allowed standalone flags: --help, --version, -V, -h

**Examples:**

- `rustup target list`
- `rustup target list --installed`
- `rustup target add wasm32-unknown-unknown`
- `rustup target remove wasm32-unknown-unknown`
- `rustup component list`
- `rustup toolchain list`
- `rustup show`

### `sccache`
<p class="cmd-url"><a href="https://github.com/mozilla/sccache">https://github.com/mozilla/sccache</a></p>

- Allowed standalone flags: --dist-status, --help, --show-adv-stats, --show-stats, --start-server, --stop-server, --version, --zero-stats, -V, -h, -s, -z
- Allowed valued flags: --stats-format

### `sqlx`
<p class="cmd-url"><a href="https://github.com/launchbadge/sqlx/tree/main/sqlx-cli">https://github.com/launchbadge/sqlx/tree/main/sqlx-cli</a></p>

- **completions**: Flags: --help, -h. Positional args accepted
- **help**: Positional args accepted
- **migrate add**: Flags: --help, --reversible, --sequential, --source, --timestamp, -h, -r, -s. Valued: --source
- **migrate build-script**: Flags: --force, --help, -h. Valued: --source
- **migrate info**: Flags: --help, -h. Valued: --database-url, --source, -D
- Allowed standalone flags: --help, --version, -h, -V

### `starship`
<p class="cmd-url"><a href="https://starship.rs/">https://starship.rs/</a></p>

- **bug-report**: Flags: --help, -h
- **completions**: Flags: --help, -h. Positional args accepted
- **explain**: Flags: --help, -h
- **help**: Positional args accepted
- **init**: Flags: --help, --print-full-init, -h. Positional args accepted
- **module**: Flags: --help, --list, -h, -l. Positional args accepted
- **preset**: Flags: --help, --list, -h, -l. Valued: --output, -o. Positional args accepted
- **print-config**: Flags: --default, --help, -d, -h. Valued: --name
- **prompt**: Flags: --help, -h. Valued: --cmd-duration, --continuation, --jobs, --keymap, --logical-path, --path, --pipestatus, --profile, --right, --status, --target, --terminal-width
- **session**: Flags: --help, -h
- **timings**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h, -V

**Examples:**

- `eval "$(starship init bash)"`
- `eval "$(starship init zsh)"`
- `eval "$(starship init fish --print-full-init)"`

