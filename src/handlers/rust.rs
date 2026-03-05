use crate::parse::{Segment, Token, WordSet, has_flag};
use crate::policy::{self, FlagPolicy};

static CARGO_BUILD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all-features", "--all-targets", "--build-plan", "--frozen",
        "--future-incompat-report", "--ignore-rust-version", "--keep-going",
        "--locked", "--no-default-features", "--offline", "--release",
        "--timings", "--unit-graph",
    ]),
    standalone_short: b"qv",
    valued: WordSet::new(&[
        "--bench", "--bin", "--color", "--config", "--example",
        "--features", "--jobs", "--manifest-path", "--message-format",
        "--package", "--profile", "--target", "--target-dir", "--test",
    ]),
    valued_short: b"jZp",
    bare: true,
    max_positional: None,
};

static CARGO_TEST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all-features", "--all-targets", "--doc", "--frozen",
        "--future-incompat-report", "--ignore-rust-version", "--keep-going",
        "--locked", "--no-default-features", "--no-fail-fast", "--no-run",
        "--offline", "--release", "--timings", "--unit-graph",
    ]),
    standalone_short: b"qv",
    valued: WordSet::new(&[
        "--bench", "--bin", "--color", "--config", "--example",
        "--features", "--jobs", "--manifest-path", "--message-format",
        "--package", "--profile", "--target", "--target-dir", "--test",
    ]),
    valued_short: b"jZp",
    bare: true,
    max_positional: None,
};

static CARGO_CHECK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all-features", "--all-targets", "--frozen",
        "--future-incompat-report", "--ignore-rust-version", "--keep-going",
        "--locked", "--no-default-features", "--offline", "--release",
        "--timings", "--unit-graph",
    ]),
    standalone_short: b"qv",
    valued: WordSet::new(&[
        "--bench", "--bin", "--color", "--config", "--example",
        "--features", "--jobs", "--manifest-path", "--message-format",
        "--package", "--profile", "--target", "--target-dir", "--test",
    ]),
    valued_short: b"jZp",
    bare: true,
    max_positional: None,
};

static CARGO_CLIPPY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all-features", "--all-targets", "--frozen",
        "--future-incompat-report", "--ignore-rust-version", "--keep-going",
        "--locked", "--no-default-features", "--no-deps", "--offline",
        "--release", "--timings", "--unit-graph",
    ]),
    standalone_short: b"qv",
    valued: WordSet::new(&[
        "--bench", "--bin", "--color", "--config", "--example",
        "--features", "--jobs", "--manifest-path", "--message-format",
        "--package", "--profile", "--target", "--target-dir", "--test",
    ]),
    valued_short: b"jZp",
    bare: true,
    max_positional: None,
};

static CARGO_BENCH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all-features", "--all-targets", "--frozen",
        "--future-incompat-report", "--ignore-rust-version", "--keep-going",
        "--locked", "--no-default-features", "--no-fail-fast", "--no-run",
        "--offline", "--release", "--timings", "--unit-graph",
    ]),
    standalone_short: b"qv",
    valued: WordSet::new(&[
        "--bench", "--bin", "--color", "--config", "--example",
        "--features", "--jobs", "--manifest-path", "--message-format",
        "--package", "--profile", "--target", "--target-dir", "--test",
    ]),
    valued_short: b"jZp",
    bare: true,
    max_positional: None,
};

static CARGO_DOC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all-features", "--document-private-items", "--frozen",
        "--future-incompat-report", "--ignore-rust-version", "--keep-going",
        "--locked", "--no-default-features", "--no-deps", "--offline",
        "--open", "--release", "--timings", "--unit-graph",
    ]),
    standalone_short: b"qv",
    valued: WordSet::new(&[
        "--bin", "--color", "--config", "--example",
        "--features", "--jobs", "--manifest-path", "--message-format",
        "--package", "--profile", "--target", "--target-dir",
    ]),
    valued_short: b"jZp",
    bare: true,
    max_positional: None,
};

static CARGO_TREE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all-features", "--duplicates", "--frozen",
        "--ignore-rust-version", "--locked", "--no-dedupe",
        "--no-default-features", "--offline",
    ]),
    standalone_short: b"deiqv",
    valued: WordSet::new(&[
        "--charset", "--color", "--config", "--depth",
        "--edges", "--features", "--format", "--invert",
        "--manifest-path", "--package", "--prefix", "--prune",
        "--target",
    ]),
    valued_short: b"p",
    bare: true,
    max_positional: None,
};

static CARGO_METADATA_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all-features", "--frozen", "--locked",
        "--no-default-features", "--no-deps", "--offline",
    ]),
    standalone_short: b"qv",
    valued: WordSet::new(&[
        "--color", "--config", "--features",
        "--filter-platform", "--format-version",
        "--manifest-path",
    ]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static CARGO_SEARCH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--frozen", "--locked", "--offline",
    ]),
    standalone_short: b"qv",
    valued: WordSet::new(&[
        "--color", "--config", "--index", "--limit",
        "--registry",
    ]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static CARGO_AUDIT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--deny", "--json", "--no-fetch", "--stale",
    ]),
    standalone_short: b"nqv",
    valued: WordSet::new(&[
        "--color", "--db", "--file", "--ignore",
        "--target-arch", "--target-os",
    ]),
    valued_short: b"f",
    bare: true,
    max_positional: None,
};

static CARGO_DENY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all-features", "--no-default-features",
    ]),
    standalone_short: b"qv",
    valued: WordSet::new(&[
        "--color", "--config", "--exclude", "--features",
        "--format", "--manifest-path", "--target",
        "--workspace",
    ]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static CARGO_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--frozen", "--locked", "--offline",
    ]),
    standalone_short: b"qv",
    valued: WordSet::new(&[
        "--color", "--config", "--manifest-path",
    ]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static CARGO_FMT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--all", "--check"]),
    standalone_short: b"qv",
    valued: WordSet::new(&[
        "--manifest-path", "--message-format", "--package",
    ]),
    valued_short: b"p",
    bare: false,
    max_positional: None,
};

static CARGO_PACKAGE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all-features", "--frozen", "--keep-going",
        "--list", "--locked", "--no-default-features",
        "--no-metadata", "--offline", "--workspace",
    ]),
    standalone_short: b"lqv",
    valued: WordSet::new(&[
        "--color", "--config", "--exclude", "--features",
        "--jobs", "--manifest-path", "--message-format",
        "--package", "--target", "--target-dir",
    ]),
    valued_short: b"jFZp",
    bare: false,
    max_positional: None,
};

static CARGO_PUBLISH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all-features", "--dry-run", "--frozen",
        "--keep-going", "--locked", "--no-default-features",
        "--offline", "--workspace",
    ]),
    standalone_short: b"nqv",
    valued: WordSet::new(&[
        "--color", "--config", "--exclude", "--features",
        "--index", "--jobs", "--manifest-path",
        "--package", "--registry", "--target",
        "--target-dir",
    ]),
    valued_short: b"jFZp",
    bare: false,
    max_positional: None,
};

pub fn is_safe_cargo(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let sub = if tokens[1].starts_with('+') { 2 } else { 1 };
    if tokens.len() < sub + 1 {
        return false;
    }
    let rest = &tokens[sub..];
    let subcmd = rest[0].as_str();
    match subcmd {
        "build" => policy::check(rest, &CARGO_BUILD_POLICY),
        "test" => policy::check(rest, &CARGO_TEST_POLICY),
        "check" => policy::check(rest, &CARGO_CHECK_POLICY),
        "clippy" => policy::check(rest, &CARGO_CLIPPY_POLICY),
        "bench" => policy::check(rest, &CARGO_BENCH_POLICY),
        "doc" => policy::check(rest, &CARGO_DOC_POLICY),
        "tree" => policy::check(rest, &CARGO_TREE_POLICY),
        "metadata" => policy::check(rest, &CARGO_METADATA_POLICY),
        "search" => policy::check(rest, &CARGO_SEARCH_POLICY),
        "audit" => policy::check(rest, &CARGO_AUDIT_POLICY),
        "deny" => policy::check(rest, &CARGO_DENY_POLICY),
        "license" | "locate-project" | "pkgid" | "read-manifest" | "verify-project" => {
            policy::check(rest, &CARGO_SIMPLE_POLICY)
        }
        "fmt" => has_flag(rest, None, Some("--check")) && policy::check(rest, &CARGO_FMT_POLICY),
        "package" => has_flag(rest, Some("-l"), Some("--list")) && policy::check(rest, &CARGO_PACKAGE_POLICY),
        "publish" => has_flag(rest, Some("-n"), Some("--dry-run")) && policy::check(rest, &CARGO_PUBLISH_POLICY),
        _ => false,
    }
}

static RUSTUP_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--installed"]),
    standalone_short: b"v",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static RUSTUP_WHICH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&["--toolchain"]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static RUSTUP_DOC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--alloc", "--book", "--cargo", "--core", "--edition-guide",
        "--embedded-book", "--nomicon", "--path", "--proc_macro",
        "--reference", "--rust-by-example", "--rustc", "--rustdoc",
        "--std", "--test", "--unstable-book",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&["--toolchain"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static RUSTUP_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--installed"]),
    standalone_short: b"v",
    valued: WordSet::new(&["--toolchain"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_rustup(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    match tokens[1].as_str() {
        "show" => policy::check(&tokens[1..], &RUSTUP_SHOW_POLICY),
        "which" => policy::check(&tokens[1..], &RUSTUP_WHICH_POLICY),
        "doc" => policy::check(&tokens[1..], &RUSTUP_DOC_POLICY),
        "component" => {
            tokens.get(2).is_some_and(|a| a == "list")
                && policy::check(&tokens[2..], &RUSTUP_LIST_POLICY)
        }
        "target" => {
            tokens.get(2).is_some_and(|a| a == "list")
                && policy::check(&tokens[2..], &RUSTUP_LIST_POLICY)
        }
        "toolchain" => {
            tokens.get(2).is_some_and(|a| a == "list")
                && policy::check(&tokens[2..], &RUSTUP_LIST_POLICY)
        }
        "run" => {
            if tokens.len() >= 4 {
                let inner = Token::join(&tokens[3..]);
                is_safe(&inner)
            } else {
                false
            }
        }
        _ => false,
    }
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "cargo" => Some(is_safe_cargo(tokens)),
        "rustup" => Some(is_safe_rustup(tokens, is_safe)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("cargo",
            "Subcommands: audit, bench, build, check, clippy, deny, doc, license, \
             locate-project, metadata, pkgid, read-manifest, search, test, tree, \
             verify-project. \
             fmt (requires --check), package (requires --list), \
             publish (requires --dry-run). \
             +toolchain selectors (e.g. +nightly) are skipped."),
        CommandDoc::handler("rustup",
            "Subcommands: doc, show, which. Multi-level: component list, \
             target list, toolchain list. \
             run <toolchain> delegates to inner command validation."),
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        cargo_clippy: "cargo clippy -- -D warnings",
        cargo_test: "cargo test",
        cargo_test_release: "cargo test --release",
        cargo_test_no_run: "cargo test --no-run",
        cargo_test_no_fail_fast: "cargo test --no-fail-fast",
        cargo_test_doc: "cargo test --doc",
        cargo_test_features: "cargo test --features serde",
        cargo_test_all_features: "cargo test --all-features",
        cargo_test_package: "cargo test --package my-crate",
        cargo_test_target: "cargo test --target x86_64-unknown-linux-gnu",
        cargo_test_jobs: "cargo test --jobs 4",
        cargo_build: "cargo build --release",
        cargo_build_all_targets: "cargo build --all-targets",
        cargo_build_locked: "cargo build --locked",
        cargo_build_offline: "cargo build --offline",
        cargo_build_frozen: "cargo build --frozen",
        cargo_build_verbose: "cargo build -v",
        cargo_build_quiet: "cargo build -q",
        cargo_check: "cargo check",
        cargo_check_all_features: "cargo check --all-features",
        cargo_doc: "cargo doc",
        cargo_doc_no_deps: "cargo doc --no-deps",
        cargo_doc_open: "cargo doc --open",
        cargo_doc_private: "cargo doc --document-private-items",
        cargo_search: "cargo search serde",
        cargo_version: "cargo --version",
        cargo_bench: "cargo bench",
        cargo_bench_no_run: "cargo bench --no-run",
        cargo_tree: "cargo tree",
        cargo_tree_duplicates: "cargo tree --duplicates",
        cargo_tree_depth: "cargo tree --depth 3",
        cargo_tree_edges: "cargo tree --edges features",
        cargo_tree_invert: "cargo tree --invert serde",
        cargo_tree_format: "cargo tree --format '{p} {l}'",
        cargo_metadata: "cargo metadata --format-version 1",
        cargo_metadata_no_deps: "cargo metadata --no-deps",
        cargo_verify_project: "cargo verify-project",
        cargo_pkgid: "cargo pkgid",
        cargo_locate_project: "cargo locate-project",
        cargo_read_manifest: "cargo read-manifest",
        cargo_audit: "cargo audit",
        cargo_audit_json: "cargo audit --json",
        cargo_audit_no_fetch: "cargo audit --no-fetch",
        cargo_deny: "cargo deny check",
        cargo_deny_licenses: "cargo deny check licenses",
        cargo_license: "cargo license",
        cargo_fmt_check: "cargo fmt --check",
        cargo_help: "cargo --help",
        cargo_install_help: "cargo install --help",
        cargo_package_list: "cargo package --list",
        cargo_package_list_redirect: "cargo package --list 2>&1",
        cargo_publish_dry_run: "cargo publish --dry-run",
        cargo_publish_dry_run_redirect: "cargo publish --dry-run 2>&1",
        cargo_run_help_safe: "cargo run --help",
        cargo_nightly_check: "cargo +nightly check",
        cargo_stable_test: "cargo +stable test",
        cargo_pinned_build: "cargo +1.91 build --release",
        cargo_nightly_clippy: "cargo +nightly clippy -- -D warnings",
        cargo_nightly_fmt_check: "cargo +nightly fmt --check",
        cargo_nightly_publish_dry_run: "cargo +nightly publish --dry-run",
        cargo_nightly_install_help: "cargo +nightly install --help",
        cargo_nightly_package_list: "cargo +nightly package --list",
        cargo_clippy_no_deps: "cargo clippy --no-deps",
        rustup_show: "rustup show",
        rustup_show_installed: "rustup show --installed",
        rustup_which: "rustup which rustc",
        rustup_which_toolchain: "rustup which --toolchain nightly rustc",
        rustup_doc: "rustup doc",
        rustup_doc_std: "rustup doc --std",
        rustup_doc_book: "rustup doc --book",
        rustup_doc_path: "rustup doc --path",
        rustup_version: "rustup --version",
        rustup_component_list: "rustup component list",
        rustup_component_list_installed: "rustup component list --installed",
        rustup_target_list: "rustup target list",
        rustup_target_list_installed: "rustup target list --installed",
        rustup_toolchain_list: "rustup toolchain list",
        rustup_toolchain_list_verbose: "rustup toolchain list -v",
        rustup_run_rustc_version: "rustup run stable rustc --version",
        rustup_run_cargo_test: "rustup run nightly cargo test",
        rustup_run_cargo_clippy: "rustup run stable cargo clippy -- -D warnings",
        rustup_run_cargo_fmt_check: "rustup run nightly cargo fmt --check",
        rustup_run_env_cargo_test: "rustup run stable env FOO=bar cargo test",
    }

    denied! {
        cargo_fmt_denied: "cargo fmt",
        cargo_install_denied: "cargo install --path .",
        cargo_run_denied: "cargo run",
        cargo_clean_denied: "cargo clean",
        cargo_package_denied: "cargo package",
        cargo_publish_dry_run_force_denied: "cargo publish --dry-run --force",
        cargo_publish_no_verify_denied: "cargo publish --dry-run --no-verify",
        cargo_publish_denied: "cargo publish",
        cargo_run_double_dash_help_denied: "cargo run -- --help",
        cargo_nightly_fmt_denied: "cargo +nightly fmt",
        cargo_nightly_publish_denied: "cargo +nightly publish",
        cargo_nightly_run_denied: "cargo +nightly run",
        cargo_bare_toolchain_denied: "cargo +nightly",
        cargo_test_unknown_denied: "cargo test --unknown-flag",
        cargo_build_unknown_denied: "cargo build --exec evil",
        cargo_check_unknown_denied: "cargo check --unknown",
        cargo_tree_unknown_denied: "cargo tree --unknown",
        cargo_metadata_unknown_denied: "cargo metadata --unknown",
        cargo_search_unknown_denied: "cargo search serde --unknown",
        cargo_audit_unknown_denied: "cargo audit --unknown",
        cargo_deny_graph_denied: "cargo deny check --graph /tmp/out.dot",
        cargo_deny_unknown_denied: "cargo deny check --unknown",
        cargo_clippy_fix_denied: "cargo clippy --fix",
        rustup_install_denied: "rustup install stable",
        rustup_update_denied: "rustup update",
        rustup_default_denied: "rustup default nightly",
        rustup_component_add_denied: "rustup component add clippy",
        rustup_self_denied: "rustup self update",
        rustup_run_cargo_fmt_denied: "rustup run nightly cargo fmt",
        rustup_run_unsafe_inner_denied: "rustup run stable rm -rf /",
        rustup_run_no_inner_denied: "rustup run stable",
        rustup_run_no_toolchain_denied: "rustup run",
        rustup_run_cargo_publish_denied: "rustup run nightly cargo publish",
        rustup_run_bash_c_denied: "rustup run stable bash -c 'rm -rf /'",
        rustup_run_env_unsafe_denied: "rustup run stable env rm foo",
        rustup_run_nested_denied: "rustup run nightly rustup run stable rm -rf /",
        rustup_show_unknown_denied: "rustup show --unknown",
        rustup_which_unknown_denied: "rustup which --unknown rustc",
        rustup_doc_unknown_denied: "rustup doc --unknown",
        rustup_component_list_unknown_denied: "rustup component list --unknown",
        rustup_target_list_unknown_denied: "rustup target list --unknown",
        rustup_toolchain_list_unknown_denied: "rustup toolchain list --unknown",
    }
}
