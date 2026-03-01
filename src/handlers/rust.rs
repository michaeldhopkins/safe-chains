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

    #[test]
    fn cargo_clippy() {
        assert!(check("cargo clippy -- -D warnings"));
    }

    #[test]
    fn cargo_test() {
        assert!(check("cargo test"));
    }

    #[test]
    fn cargo_build() {
        assert!(check("cargo build --release"));
    }

    #[test]
    fn cargo_check() {
        assert!(check("cargo check"));
    }

    #[test]
    fn cargo_doc() {
        assert!(check("cargo doc"));
    }

    #[test]
    fn cargo_search() {
        assert!(check("cargo search serde"));
    }

    #[test]
    fn cargo_version() {
        assert!(check("cargo --version"));
    }

    #[test]
    fn cargo_bench() {
        assert!(check("cargo bench"));
    }

    #[test]
    fn cargo_tree() {
        assert!(check("cargo tree"));
    }

    #[test]
    fn cargo_metadata() {
        assert!(check("cargo metadata --format-version 1"));
    }

    #[test]
    fn cargo_verify_project() {
        assert!(check("cargo verify-project"));
    }

    #[test]
    fn cargo_pkgid() {
        assert!(check("cargo pkgid"));
    }

    #[test]
    fn cargo_locate_project() {
        assert!(check("cargo locate-project"));
    }

    #[test]
    fn cargo_read_manifest() {
        assert!(check("cargo read-manifest"));
    }

    #[test]
    fn cargo_audit() {
        assert!(check("cargo audit"));
    }

    #[test]
    fn cargo_deny() {
        assert!(check("cargo deny check"));
    }

    #[test]
    fn cargo_license() {
        assert!(check("cargo license"));
    }

    #[test]
    fn cargo_fmt_check() {
        assert!(check("cargo fmt --check"));
    }

    #[test]
    fn cargo_fmt_denied() {
        assert!(!check("cargo fmt"));
    }

    #[test]
    fn cargo_install_denied() {
        assert!(!check("cargo install --path ."));
    }

    #[test]
    fn cargo_run_denied() {
        assert!(!check("cargo run"));
    }

    #[test]
    fn cargo_clean_denied() {
        assert!(!check("cargo clean"));
    }

    #[test]
    fn cargo_help() {
        assert!(check("cargo --help"));
    }

    #[test]
    fn cargo_install_help() {
        assert!(check("cargo install --help"));
    }

    #[test]
    fn cargo_package_list() {
        assert!(check("cargo package --list"));
    }

    #[test]
    fn cargo_package_list_redirect() {
        assert!(check("cargo package --list 2>&1"));
    }

    #[test]
    fn cargo_package_denied() {
        assert!(!check("cargo package"));
    }

    #[test]
    fn cargo_publish_dry_run() {
        assert!(check("cargo publish --dry-run"));
    }

    #[test]
    fn cargo_publish_dry_run_redirect() {
        assert!(check("cargo publish --dry-run 2>&1"));
    }

    #[test]
    fn cargo_publish_dry_run_force_denied() {
        assert!(!check("cargo publish --dry-run --force"));
    }

    #[test]
    fn cargo_publish_no_verify_denied() {
        assert!(!check("cargo publish --dry-run --no-verify"));
    }

    #[test]
    fn cargo_publish_denied() {
        assert!(!check("cargo publish"));
    }

    #[test]
    fn cargo_run_double_dash_help_denied() {
        assert!(!check("cargo run -- --help"));
    }

    #[test]
    fn cargo_run_help_safe() {
        assert!(check("cargo run --help"));
    }

    #[test]
    fn cargo_nightly_check() {
        assert!(check("cargo +nightly check"));
    }

    #[test]
    fn cargo_stable_test() {
        assert!(check("cargo +stable test"));
    }

    #[test]
    fn cargo_pinned_build() {
        assert!(check("cargo +1.91 build --release"));
    }

    #[test]
    fn cargo_nightly_clippy() {
        assert!(check("cargo +nightly clippy -- -D warnings"));
    }

    #[test]
    fn cargo_nightly_fmt_check() {
        assert!(check("cargo +nightly fmt --check"));
    }

    #[test]
    fn cargo_nightly_fmt_denied() {
        assert!(!check("cargo +nightly fmt"));
    }

    #[test]
    fn cargo_nightly_publish_dry_run() {
        assert!(check("cargo +nightly publish --dry-run"));
    }

    #[test]
    fn cargo_nightly_publish_denied() {
        assert!(!check("cargo +nightly publish"));
    }

    #[test]
    fn cargo_nightly_run_denied() {
        assert!(!check("cargo +nightly run"));
    }

    #[test]
    fn cargo_nightly_install_help() {
        assert!(check("cargo +nightly install --help"));
    }

    #[test]
    fn cargo_nightly_package_list() {
        assert!(check("cargo +nightly package --list"));
    }

    #[test]
    fn cargo_bare_toolchain_denied() {
        assert!(!check("cargo +nightly"));
    }

    #[test]
    fn rustup_show() {
        assert!(check("rustup show"));
    }

    #[test]
    fn rustup_which() {
        assert!(check("rustup which rustc"));
    }

    #[test]
    fn rustup_doc() {
        assert!(check("rustup doc"));
    }

    #[test]
    fn rustup_version() {
        assert!(check("rustup --version"));
    }

    #[test]
    fn rustup_component_list() {
        assert!(check("rustup component list"));
    }

    #[test]
    fn rustup_target_list() {
        assert!(check("rustup target list"));
    }

    #[test]
    fn rustup_toolchain_list() {
        assert!(check("rustup toolchain list"));
    }

    #[test]
    fn rustup_install_denied() {
        assert!(!check("rustup install stable"));
    }

    #[test]
    fn rustup_update_denied() {
        assert!(!check("rustup update"));
    }

    #[test]
    fn rustup_default_denied() {
        assert!(!check("rustup default nightly"));
    }

    #[test]
    fn rustup_component_add_denied() {
        assert!(!check("rustup component add clippy"));
    }

    #[test]
    fn rustup_self_denied() {
        assert!(!check("rustup self update"));
    }

    #[test]
    fn rustup_run_rustc_version() {
        assert!(check("rustup run stable rustc --version"));
    }

    #[test]
    fn rustup_run_cargo_test() {
        assert!(check("rustup run nightly cargo test"));
    }

    #[test]
    fn rustup_run_cargo_clippy() {
        assert!(check("rustup run stable cargo clippy -- -D warnings"));
    }

    #[test]
    fn rustup_run_cargo_fmt_check() {
        assert!(check("rustup run nightly cargo fmt --check"));
    }

    #[test]
    fn rustup_run_cargo_fmt_denied() {
        assert!(!check("rustup run nightly cargo fmt"));
    }

    #[test]
    fn rustup_run_unsafe_inner_denied() {
        assert!(!check("rustup run stable rm -rf /"));
    }

    #[test]
    fn rustup_run_no_inner_denied() {
        assert!(!check("rustup run stable"));
    }

    #[test]
    fn rustup_run_no_toolchain_denied() {
        assert!(!check("rustup run"));
    }

    #[test]
    fn rustup_run_cargo_publish_denied() {
        assert!(!check("rustup run nightly cargo publish"));
    }

    #[test]
    fn rustup_run_bash_c_denied() {
        assert!(!check("rustup run stable bash -c 'rm -rf /'"));
    }

    #[test]
    fn rustup_run_env_unsafe_denied() {
        assert!(!check("rustup run stable env rm foo"));
    }

    #[test]
    fn rustup_run_nested_denied() {
        assert!(!check("rustup run nightly rustup run stable rm -rf /"));
    }

    #[test]
    fn rustup_run_env_cargo_test() {
        assert!(check("rustup run stable env FOO=bar cargo test"));
    }
}
