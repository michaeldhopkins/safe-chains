use crate::command::{CommandDef, SubDef};
use crate::parse::{Segment, Token};
use crate::policy::{FlagPolicy, FlagStyle};

use crate::parse::WordSet;

macro_rules! cargo_compile_policy {
    ([$($standalone:literal),* $(,)?]) => {
        FlagPolicy {
            standalone: WordSet::new(&[$($standalone),*]),
            standalone_short: b"qv",
            valued: WordSet::new(&[
                "--bench", "--bin", "--color", "--config", "--example",
                "--features", "--jobs", "--manifest-path", "--message-format",
                "--package", "--profile", "--target", "--target-dir", "--test",
            ]),
            valued_short: b"jZp",
            bare: true,
            max_positional: None,
            flag_style: FlagStyle::Strict,
        }
    };
}

static CARGO_BUILD_POLICY: FlagPolicy = cargo_compile_policy!([
    "--all-features", "--all-targets", "--benches", "--bins", "--build-plan",
    "--examples", "--frozen", "--future-incompat-report", "--ignore-rust-version",
    "--keep-going", "--lib", "--locked", "--no-default-features", "--offline",
    "--release", "--tests", "--timings", "--unit-graph",
]);

static CARGO_TEST_POLICY: FlagPolicy = cargo_compile_policy!([
    "--all-features", "--all-targets", "--benches", "--bins", "--doc",
    "--examples", "--frozen", "--future-incompat-report", "--ignore-rust-version",
    "--keep-going", "--lib", "--locked", "--no-default-features", "--no-fail-fast",
    "--no-run", "--offline", "--release", "--tests", "--timings", "--unit-graph",
]);

static CARGO_CHECK_POLICY: FlagPolicy = cargo_compile_policy!([
    "--all-features", "--all-targets", "--benches", "--bins", "--examples",
    "--frozen", "--future-incompat-report", "--ignore-rust-version",
    "--keep-going", "--lib", "--locked", "--no-default-features", "--offline",
    "--release", "--tests", "--timings", "--unit-graph",
]);

static CARGO_CLIPPY_POLICY: FlagPolicy = cargo_compile_policy!([
    "--all-features", "--all-targets", "--benches", "--bins", "--examples",
    "--frozen", "--future-incompat-report", "--ignore-rust-version",
    "--keep-going", "--lib", "--locked", "--no-default-features", "--no-deps",
    "--offline", "--release", "--tests", "--timings", "--unit-graph",
]);

static CARGO_DOC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all-features", "--bins", "--document-private-items", "--examples",
        "--frozen", "--future-incompat-report", "--ignore-rust-version",
        "--keep-going", "--locked", "--no-default-features", "--no-deps",
        "--offline", "--open", "--release", "--timings", "--unit-graph",
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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
};

static CARGO_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--frozen", "--locked", "--offline",
    ]),
    standalone_short: b"qv",
    valued: WordSet::new(&[
        "--color", "--config", "--index", "--registry",
    ]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
};

fn check_cargo_sub(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    let sub = usize::from(!tokens.is_empty() && tokens[0].starts_with('+'));
    if tokens.len() < sub + 1 {
        return false;
    }
    let rest = &tokens[sub..];
    CARGO_SUBS
        .iter()
        .find(|s| s.name() == rest[0].as_str())
        .is_some_and(|s| s.check(rest, is_safe))
}

static CARGO_HELP_SUB_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(1),
    flag_style: FlagStyle::Positional,
};

static CARGO_HELP_ONLY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static CARGO_SUBS: &[SubDef] = &[
    SubDef::Policy { name: "audit", policy: &CARGO_AUDIT_POLICY },
    SubDef::Policy { name: "bench", policy: &CARGO_TEST_POLICY },
    SubDef::Policy { name: "build", policy: &CARGO_BUILD_POLICY },
    SubDef::Policy { name: "check", policy: &CARGO_CHECK_POLICY },
    SubDef::Policy { name: "clippy", policy: &CARGO_CLIPPY_POLICY },
    SubDef::Policy { name: "deny", policy: &CARGO_DENY_POLICY },
    SubDef::Policy { name: "doc", policy: &CARGO_DOC_POLICY },
    SubDef::Policy { name: "help", policy: &CARGO_HELP_SUB_POLICY },
    SubDef::Guarded {
        name: "fmt",
        guard_short: None,
        guard_long: "--check",
        policy: &CARGO_FMT_POLICY,
    },
    SubDef::Policy { name: "info", policy: &CARGO_INFO_POLICY },
    SubDef::Policy { name: "install", policy: &CARGO_HELP_ONLY_POLICY },
    SubDef::Policy { name: "license", policy: &CARGO_SIMPLE_POLICY },
    SubDef::Policy { name: "locate-project", policy: &CARGO_SIMPLE_POLICY },
    SubDef::Policy { name: "metadata", policy: &CARGO_METADATA_POLICY },
    SubDef::Guarded {
        name: "package",
        guard_short: Some("-l"),
        guard_long: "--list",
        policy: &CARGO_PACKAGE_POLICY,
    },
    SubDef::Policy { name: "pkgid", policy: &CARGO_SIMPLE_POLICY },
    SubDef::Guarded {
        name: "publish",
        guard_short: Some("-n"),
        guard_long: "--dry-run",
        policy: &CARGO_PUBLISH_POLICY,
    },
    SubDef::Policy { name: "read-manifest", policy: &CARGO_SIMPLE_POLICY },
    SubDef::Policy { name: "run", policy: &CARGO_HELP_ONLY_POLICY },
    SubDef::Policy { name: "search", policy: &CARGO_SEARCH_POLICY },
    SubDef::Policy { name: "test", policy: &CARGO_TEST_POLICY },
    SubDef::Policy { name: "tree", policy: &CARGO_TREE_POLICY },
    SubDef::Policy { name: "verify-project", policy: &CARGO_SIMPLE_POLICY },
];

pub(crate) static CARGO: CommandDef = CommandDef {
    name: "cargo",
    subs: CARGO_SUBS,
    bare_flags: &[],
    help_eligible: true,
    url: "https://doc.rust-lang.org/cargo/commands/",
};

pub(in crate::handlers::rust) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "cargo" => {
            if tokens.len() < 2 {
                return Some(false);
            }
            let sub = if tokens[1].starts_with('+') { 2 } else { 1 };
            if tokens.len() < sub + 1 {
                return Some(false);
            }
            let arg = tokens[sub].as_str();
            if tokens.len() == sub + 1 && (arg == "--help" || arg == "--version") {
                return Some(true);
            }
            Some(check_cargo_sub(&tokens[sub..], is_safe))
        }
        _ => None,
    }
}

pub(in crate::handlers::rust) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut cargo_doc = CARGO.to_doc();
    cargo_doc.description.push_str("\n\n+toolchain selectors (e.g. +nightly) are skipped.");
    vec![cargo_doc]
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
        cargo_help_sub: "cargo help clippy",
        cargo_help_sub_build: "cargo help build",
        cargo_help_bare: "cargo help",
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
        cargo_clippy_lib: "cargo clippy --lib",
        cargo_clippy_tests: "cargo clippy --tests",
        cargo_clippy_tests_warnings: "cargo clippy --tests -- -D warnings",
        cargo_test_lib: "cargo test --lib",
        cargo_test_tests: "cargo test --tests",
        cargo_test_bins: "cargo test --bins",
        cargo_check_lib: "cargo check --lib",
        cargo_check_benches: "cargo check --benches",
        cargo_build_lib: "cargo build --lib",
        cargo_build_examples: "cargo build --examples",
        cargo_bench_lib: "cargo bench --lib",
        cargo_bench_bins: "cargo bench --bins",
        cargo_doc_bins: "cargo doc --bins",
        cargo_doc_examples: "cargo doc --examples",
        cargo_info: "cargo info serde",
        cargo_info_registry: "cargo info serde --registry crates-io",
    }

    denied! {
        cargo_fmt_denied: "cargo fmt",
        cargo_package_denied: "cargo package",
        cargo_publish_dry_run_force_denied: "cargo publish --dry-run --force",
        cargo_publish_no_verify_denied: "cargo publish --dry-run --no-verify",
        cargo_publish_denied: "cargo publish",
        cargo_run_double_dash_help_denied: "cargo run -- --help",
        cargo_nightly_fmt_denied: "cargo +nightly fmt",
        cargo_nightly_publish_denied: "cargo +nightly publish",
        cargo_nightly_run_denied: "cargo +nightly run",
        cargo_bare_toolchain_denied: "cargo +nightly",
        cargo_deny_graph_denied: "cargo deny check --graph /tmp/out.dot",
        cargo_info_bare_denied: "cargo info",
        cargo_clippy_fix_denied: "cargo clippy --fix",
    }
}
