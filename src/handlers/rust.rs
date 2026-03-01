use crate::parse::{FlagCheck, Segment, Token, WordSet};

static CARGO_SAFE: WordSet = WordSet::new(&[
    "--version", "audit", "bench", "build", "check", "clippy", "deny",
    "doc", "license", "locate-project", "metadata", "pkgid",
    "read-manifest", "search", "test", "tree", "verify-project",
]);

static CARGO_FMT: FlagCheck =
    FlagCheck::new(&["--check"], &[]);

static CARGO_PACKAGE_LIST: FlagCheck =
    FlagCheck::new(&["--list"], &[]);

static CARGO_PUBLISH_DRY: FlagCheck =
    FlagCheck::new(&["--dry-run"], &["--force", "--no-verify"]);

static RUSTUP_SAFE: WordSet =
    WordSet::new(&["--version", "doc", "show", "which"]);

static RUSTUP_MULTI: &[(&str, WordSet)] = &[
    ("component", WordSet::new(&["list"])),
    ("target", WordSet::new(&["list"])),
    ("toolchain", WordSet::new(&["list"])),
];

pub fn is_safe_cargo(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let sub = if tokens[1].starts_with('+') { 2 } else { 1 };
    if tokens.len() < sub + 1 {
        return false;
    }
    if tokens.last().is_some_and(|t| *t == "--help")
        && !tokens.iter().any(|t| *t == "--")
    {
        return true;
    }
    if CARGO_SAFE.contains(&tokens[sub]) {
        return true;
    }
    if tokens[sub] == "fmt" {
        return CARGO_FMT.is_safe(&tokens[sub + 1..]);
    }
    if tokens[sub] == "package" {
        return CARGO_PACKAGE_LIST.is_safe(&tokens[sub + 1..]);
    }
    if tokens[sub] == "publish" {
        return CARGO_PUBLISH_DRY.is_safe(&tokens[sub + 1..]);
    }
    false
}

pub fn is_safe_rustup(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    if super::is_safe_subcmd(tokens, &RUSTUP_SAFE, RUSTUP_MULTI) {
        return true;
    }
    if tokens.len() >= 4 && tokens[1] == "run" {
        let inner = Token::join(&tokens[3..]);
        return is_safe(&inner);
    }
    false
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, doc, doc_multi, describe_flagcheck};
    vec![
        CommandDoc::handler("cargo",
            doc(&CARGO_SAFE)
                .section(format!(
                    "Guarded: fmt ({}), package ({}), publish ({}).\n\
                     +toolchain selectors (e.g. +nightly, +stable) are skipped.\n\
                     Any subcommand with --help is safe (unless -- separator is present).",
                    describe_flagcheck(&CARGO_FMT).trim_end_matches('.'),
                    describe_flagcheck(&CARGO_PACKAGE_LIST).trim_end_matches('.'),
                    describe_flagcheck(&CARGO_PUBLISH_DRY).trim_end_matches('.'),
                ))
                .build()),
        CommandDoc::handler("rustup",
            doc_multi(&RUSTUP_SAFE, RUSTUP_MULTI)
                .section("run <toolchain> delegates to inner command validation.")
                .build()),
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
        cargo_build: "cargo build --release",
        cargo_check: "cargo check",
        cargo_doc: "cargo doc",
        cargo_search: "cargo search serde",
        cargo_version: "cargo --version",
        cargo_bench: "cargo bench",
        cargo_tree: "cargo tree",
        cargo_metadata: "cargo metadata --format-version 1",
        cargo_verify_project: "cargo verify-project",
        cargo_pkgid: "cargo pkgid",
        cargo_locate_project: "cargo locate-project",
        cargo_read_manifest: "cargo read-manifest",
        cargo_audit: "cargo audit",
        cargo_deny: "cargo deny check",
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
        rustup_show: "rustup show",
        rustup_which: "rustup which rustc",
        rustup_doc: "rustup doc",
        rustup_version: "rustup --version",
        rustup_component_list: "rustup component list",
        rustup_target_list: "rustup target list",
        rustup_toolchain_list: "rustup toolchain list",
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
    }
}
