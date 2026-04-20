# Rust

### `cargo`
<p class="cmd-url"><a href="https://doc.rust-lang.org/cargo/commands/">https://doc.rust-lang.org/cargo/commands/</a></p>

- **about generate**: Flags: --all-features, --fail, --frozen, --help, --locked, --no-default-features, --offline, --workspace, -V, -h. Valued: --color, --config, --features, --format, --manifest-path, --name, --target, --threshold, -L, -c, -m, -n
- **about**: Flags: --help, --version, -V, -h. Valued: --color, -L
- **audit**: Flags: --deny, --help, --json, --no-fetch, --stale, -h, -n, --quiet, -q, -v. Valued: --color, --db, --file, --ignore, --target-arch, --target-os, -f
- **bench**: Flags: --all-features, --all-targets, --benches, --bins, --doc, --examples, --frozen, --future-incompat-report, --help, --ignore-rust-version, --keep-going, --lib, --locked, --no-default-features, --no-fail-fast, --no-run, --offline, --release, --tests, --timings, --unit-graph, -h, --quiet, -q, -v. Valued: --bench, --bin, --color, --config, --example, --features, --jobs, --manifest-path, --message-format, --package, --profile, --target, --target-dir, --test, -Z, -j, -p
- **bloat**: Flags: --crates, --filter, --help, --lib, --no-default-features, --release, --time, --wide, -h. Valued: --bin, --example, --features, --jobs, --manifest-path, --message-format, --package, --target, -j, -n, -p
- **build**: Flags: --all-features, --all-targets, --benches, --bins, --build-plan, --examples, --frozen, --future-incompat-report, --help, --ignore-rust-version, --keep-going, --lib, --locked, --no-default-features, --offline, --release, --tests, --timings, --unit-graph, -h, --quiet, -q, -v. Valued: --bench, --bin, --color, --config, --example, --features, --jobs, --manifest-path, --message-format, --package, --profile, --target, --target-dir, --test, -Z, -j, -p
- **check**: Flags: --all-features, --all-targets, --benches, --bins, --examples, --frozen, --future-incompat-report, --help, --ignore-rust-version, --keep-going, --lib, --locked, --no-default-features, --offline, --release, --tests, --timings, --unit-graph, -h, --quiet, -q, -v. Valued: --bench, --bin, --color, --config, --example, --features, --jobs, --manifest-path, --message-format, --package, --profile, --target, --target-dir, --test, -Z, -j, -p
- **clean**: Flags: --doc, --dry-run, --frozen, --help, --locked, --offline, --release, --workspace, -h, -n, --quiet, -q, -r, -v. Valued: --color, --config, --lockfile-path, --manifest-path, --package, --profile, --target, --target-dir, -Z, -p
- **clippy**: Flags: --all-features, --all-targets, --benches, --bins, --examples, --frozen, --future-incompat-report, --help, --ignore-rust-version, --keep-going, --lib, --locked, --no-default-features, --no-deps, --offline, --release, --tests, --timings, --unit-graph, -h, --quiet, -q, -v. Valued: --bench, --bin, --color, --config, --example, --features, --jobs, --manifest-path, --message-format, --package, --profile, --target, --target-dir, --test, -Z, -j, -p
- **config get**: Flags: --frozen, --help, --locked, --offline, --show-origin, -h, --quiet, -q, -v. Valued: --color, --config, --format, --merged, -Z
- **criterion**: Flags: --all, --all-features, --all-targets, --benches, --bins, --debug, --examples, --frozen, --help, --lib, --locked, --no-default-features, --no-fail-fast, --no-run, --offline, --tests, --verbose, --version, --workspace, -V, -h, -v. Valued: --bench, --bin, --color, --criterion-manifest-path, --example, --exclude, --features, --history-description, --history-id, --jobs, --manifest-path, --message-format, --output-format, --package, --plotting-backend, --target, --target-dir, --test, -Z, -j, -p
- **cyclonedx**: Flags: --all, --all-features, --help, --license-strict, --no-default-features, --quiet, --target-in-filename, --top-level, --verbose, --version, -V, -a, -h, -q, -v. Valued: --describe, --features, --format, --license-accept-named, --manifest-path, --override-filename, --spec-version, --target, -F, -f
- **deny**: Flags: --all-features, --help, --no-default-features, -h, --quiet, -q, -v. Valued: --color, --config, --exclude, --features, --format, --manifest-path, --target, --workspace
- **doc**: Flags: --all-features, --bins, --document-private-items, --examples, --frozen, --future-incompat-report, --help, --ignore-rust-version, --keep-going, --locked, --no-default-features, --no-deps, --offline, --open, --release, --timings, --unit-graph, -h, --quiet, -q, -v. Valued: --bin, --color, --config, --example, --features, --jobs, --manifest-path, --message-format, --package, --profile, --target, --target-dir, -Z, -j, -p
- **expand**: Flags: --all-features, --help, --lib, --no-default-features, --release, --tests, --ugly, -h. Valued: --bin, --color, --example, --features, --manifest-path, --package, --target, --theme, -p
- **fetch**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --config, --lockfile-path, --manifest-path, --target, -Z
- **fmt** (requires --check): Flags: --all, --check, --help, -h, --quiet, -q, -v. Valued: --manifest-path, --message-format, --package, -p
- **geiger**: Flags: --all, --all-dependencies, --all-features, --all-targets, --build-dependencies, --dev-dependencies, --forbid-only, --frozen, --help, --include-tests, --invert, --locked, --no-default-features, --no-indent, --offline, --prefix-depth, --quiet, --verbose, --version, -V, -a, -f, -h, -i, -q, -v. Valued: --color, --features, --format, --manifest-path, --output-format, --package, --section-name, --target, -Z, -p
- **generate-lockfile**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --config, --manifest-path
- **help**: Positional args accepted
- **info**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --config, --index, --registry
- **install**: Flags: --all-features, --bins, --debug, --force, --frozen, --help, --ignore-rust-version, --keep-going, --locked, --no-default-features, --no-track, --offline, --quiet, --timings, --verbose, -f, -h, -q, -v. Valued: --bin, --color, --config, --example, --features, --jobs, --message-format, --path, --profile, --root, --target, --target-dir, -F, -Z, -j
- **license**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --config, --manifest-path
- **llvm-cov**: Flags: --all-features, --all-targets, --help, --html, --json, --lcov, --lib, --locked, --no-cfg-coverage, --no-default-features, --no-fail-fast, --no-run, --open, --release, --text, -h. Valued: --bin, --branch, --codecov, --cobertura, --color, --config, --example, --exclude, --features, --ignore-filename-regex, --ignore-run-fail, --jobs, --manifest-path, --output-dir, --output-path, --package, --profile, --target, --target-dir, --test, -j, -p
- **locate-project**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --config, --manifest-path
- **machete**: Flags: --help, --skip-target-dir, --with-metadata, -V, -h
- **metadata**: Flags: --all-features, --frozen, --help, --locked, --no-default-features, --no-deps, --offline, -h, --quiet, -q, -v. Valued: --color, --config, --features, --filter-platform, --format-version, --manifest-path
- **msrv find**: Flags: --all-features, --bisect, --help, --ignore-lockfile, --include-all-patch-releases, --linear, --no-check-feedback, --no-default-features, --no-log, --no-user-output, -h. Valued: --component, --features, --log-level, --log-target, --manifest-path, --max, --maximum, --min, --minimum, --output-format, --path, --release-source, --target
- **msrv list**: Flags: --help, --no-log, --no-user-output, -h. Valued: --log-level, --log-target, --manifest-path, --output-format, --path, --variant
- **msrv show**: Flags: --help, --no-log, --no-user-output, -h. Valued: --log-level, --log-target, --manifest-path, --output-format, --path
- **msrv verify**: Flags: --all-features, --help, --ignore-lockfile, --include-all-patch-releases, --no-check-feedback, --no-default-features, --no-log, --no-user-output, -h. Valued: --component, --features, --log-level, --log-target, --manifest-path, --max, --maximum, --min, --minimum, --output-format, --path, --release-source, --rust-version, --target
- **msrv**: Flags: --help, --no-log, --no-user-output, --version, -V, -h. Valued: --log-level, --log-target, --manifest-path, --output-format, --path
- **nextest archive**: Flags: --all-features, --help, --locked, --no-default-features, --release, -h. Valued: --archive-file, --archive-format, --cargo-profile, --features, --manifest-path, --package, --target, --target-dir, -p
- **nextest list**: Flags: --all-features, --help, --lib, --locked, --no-default-features, --release, -T, -h. Valued: --bin, --color, --config, --exclude, --features, --manifest-path, --message-format, --package, --partition, --profile, --target, --target-dir, --test, -E, -p
- **nextest run**: Flags: --all-features, --all-targets, --help, --lib, --locked, --no-capture, --no-default-features, --no-fail-fast, --release, --status-level, -h. Valued: --bin, --cargo-profile, --color, --config, --exclude, --features, --jobs, --manifest-path, --package, --partition, --profile, --retries, --target, --target-dir, --test, --test-threads, --threads, -E, -j, -p
- **nextest show-config**: Flags: --help, -h
- **outdated**: Flags: --aggressive, --color, --depth, --exit-code, --features, --help, --manifest-path, --packages, --root-deps-only, --verbose, --workspace, -R, -V, -d, -h, -n, --quiet, -q, -r, -v, -w. Valued: --color, --depth, --exclude, --features, --ignore, --manifest-path, --packages, -d, -e, -i, -p
- **package** (requires --list, -l): Flags: --all-features, --allow-dirty, --exclude-lockfile, --frozen, --help, --keep-going, --list, --locked, --no-default-features, --no-metadata, --no-verify, --offline, --workspace, -h, -l, --quiet, -q, -v. Valued: --color, --config, --exclude, --features, --index, --jobs, --lockfile-path, --manifest-path, --message-format, --package, --registry, --target, --target-dir, -F, -Z, -j, -p
- **pkgid**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --config, --manifest-path
- **public-api**: Flags: --all-features, --help, --no-default-features, --simplified, --version, -V, -h, -s. Valued: --color, --features, --manifest-path, --omit, --package, --target, -F, -p
- **publish** (requires --dry-run, -n): Flags: --all-features, --allow-dirty, --dry-run, --frozen, --help, --keep-going, --locked, --no-default-features, --no-verify, --offline, --workspace, -h, -n, --quiet, -q, -v. Valued: --color, --config, --exclude, --features, --index, --jobs, --lockfile-path, --manifest-path, --package, --registry, --target, --target-dir, -F, -Z, -j, -p
- **read-manifest**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --config, --manifest-path
- **report future-incompatibilities**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --config, --id, --package, -Z, -p
- **run**: Flags: --help, -h
- **search**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --config, --index, --limit, --registry
- **semver-checks check-release**: Flags: --all-features, --default-features, --help, --verbose, -h, -v. Valued: --baseline-rev, --baseline-root, --baseline-version, --color, --config-path, --current-rustdoc, --exclude, --features, --manifest-path, --package, --release-type, --target, -j, -p
- **test**: Flags: --all-features, --all-targets, --benches, --bins, --doc, --examples, --frozen, --future-incompat-report, --help, --ignore-rust-version, --keep-going, --lib, --locked, --no-default-features, --no-fail-fast, --no-run, --offline, --release, --tests, --timings, --unit-graph, -h, --quiet, -q, -v. Valued: --bench, --bin, --color, --config, --example, --features, --jobs, --manifest-path, --message-format, --package, --profile, --target, --target-dir, --test, -Z, -j, -p
- **tree**: Flags: --all-features, --duplicates, --frozen, --help, --ignore-rust-version, --locked, --no-dedupe, --no-default-features, --offline, -d, -e, -h, -i, --quiet, -q, -v. Valued: --charset, --color, --config, --depth, --edges, --features, --format, --invert, --manifest-path, --package, --prefix, --prune, --target, -p
- **udeps**: Flags: --all, --all-features, --all-targets, --benches, --bins, --examples, --frozen, --help, --keep-going, --lib, --locked, --no-default-features, --offline, --release, --tests, --workspace, --version, -V, -h, --quiet, -q, -v. Valued: --backend, --bench, --bin, --color, --example, --exclude, --features, --jobs, --manifest-path, --message-format, --output, --package, --profile, --target, --target-dir, --test, -j, -p
- **update** (requires --dry-run): Flags: --aggressive, --dry-run, --frozen, --help, --locked, --offline, --recursive, --workspace, -h, --quiet, -q, -v. Valued: --color, --config, --manifest-path, --package, --precise, -p
- **vendor**: Flags: --frozen, --help, --locked, --no-delete, --offline, --respect-source-config, --versioned-dirs, -h, --quiet, -q, -v. Valued: --color, --config, --lockfile-path, --manifest-path, --sync, -Z, -s
- **verify-project**: Flags: --frozen, --help, --locked, --offline, -h, --quiet, -q, -v. Valued: --color, --config, --manifest-path
- **version**: Flags: --help, -h. Positional args accepted
- **vet check**: Flags: --help, -h
- **vet dump-graph**: Flags: --help, -h. Valued: --depth
- **vet explain-audit**: Flags: --help, -h. Valued: --criteria
- **vet suggest**: Flags: --help, -h
- **vet**: Flags: --frozen, --help, --locked, --no-all-features, --no-default-features, --no-minimize-exemptions, --no-registry-suggestions, --version, -V, -h. Valued: --cache-dir, --cargo-arg, --features, --filter-graph, --manifest-path, --output-format, --store-path, --verbose
- Allowed standalone flags: --help, --version, -V, -h

### `rustup`
<p class="cmd-url"><a href="https://rust-lang.github.io/rustup/">https://rust-lang.github.io/rustup/</a></p>

- **component list**: Flags: --help, --installed, -h, -v. Valued: --toolchain
- **doc**: Flags: --alloc, --book, --cargo, --core, --edition-guide, --embedded-book, --help, --nomicon, --path, --proc_macro, --reference, --rust-by-example, --rustc, --rustdoc, --std, --test, --unstable-book, -h. Valued: --toolchain
- **show**: Flags: --help, --installed, -h, -v
- **target list**: Flags: --help, --installed, -h, -v. Valued: --toolchain
- **toolchain list**: Flags: --help, --installed, -h, -v. Valued: --toolchain
- **which**: Flags: --help, -h. Valued: --toolchain
- Allowed standalone flags: --help, --version, -V, -h

