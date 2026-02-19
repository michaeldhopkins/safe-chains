use std::collections::HashSet;
use std::sync::LazyLock;

static CARGO_SAFE: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "clippy",
        "test",
        "build",
        "check",
        "doc",
        "search",
        "--version",
        "bench",
        "tree",
        "metadata",
        "verify-project",
        "pkgid",
        "locate-project",
        "read-manifest",
        "audit",
        "deny",
    ])
});

static RUSTUP_SAFE: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["show", "which", "doc", "--version"]));

static RUSTUP_MULTI: LazyLock<Vec<(&'static str, HashSet<&'static str>)>> =
    LazyLock::new(|| {
        vec![
            ("component", HashSet::from(["list"])),
            ("target", HashSet::from(["list"])),
            ("toolchain", HashSet::from(["list"])),
        ]
    });

pub fn is_safe_cargo(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if CARGO_SAFE.contains(tokens[1].as_str()) {
        return true;
    }
    if tokens[1] == "fmt" {
        return tokens[2..].iter().any(|t| t == "--check");
    }
    false
}

pub fn is_safe_rustup(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if RUSTUP_SAFE.contains(tokens[1].as_str()) {
        return true;
    }
    for (prefix, actions) in RUSTUP_MULTI.iter() {
        if tokens[1] == *prefix {
            return tokens.get(2).is_some_and(|a| actions.contains(a.as_str()));
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::is_safe;

    fn check(cmd: &str) -> bool {
        is_safe(cmd)
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
}
