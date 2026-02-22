use crate::parse::{Token, WordSet};

static SWIFT_SAFE: WordSet =
    WordSet::new(&["--version", "build", "test"]);

static SWIFT_PACKAGE_SAFE: WordSet =
    WordSet::new(&["describe", "dump-package", "show-dependencies"]);

pub fn is_safe_swift(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if SWIFT_SAFE.contains(&tokens[1]) {
        return true;
    }
    if tokens[1] == "package" {
        return tokens
            .get(2)
            .is_some_and(|a| SWIFT_PACKAGE_SAFE.contains(a));
    }
    false
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocKind};
    vec![CommandDoc {
        name: "swift",
        kind: DocKind::Handler,
        description: "Allowed: --version, test, build. Multi-word: package describe/dump-package/show-dependencies.",
    }]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    #[test]
    fn swift_version() {
        assert!(check("swift --version"));
    }

    #[test]
    fn swift_test() {
        assert!(check("swift test"));
    }

    #[test]
    fn swift_build() {
        assert!(check("swift build"));
    }

    #[test]
    fn swift_package_describe() {
        assert!(check("swift package describe"));
    }

    #[test]
    fn swift_package_dump_package() {
        assert!(check("swift package dump-package"));
    }

    #[test]
    fn swift_package_show_dependencies() {
        assert!(check("swift package show-dependencies"));
    }

    #[test]
    fn swift_run_denied() {
        assert!(!check("swift run"));
    }

    #[test]
    fn swift_package_init_denied() {
        assert!(!check("swift package init"));
    }

    #[test]
    fn swift_package_update_denied() {
        assert!(!check("swift package update"));
    }

    #[test]
    fn swift_package_resolve_denied() {
        assert!(!check("swift package resolve"));
    }

    #[test]
    fn bare_swift_denied() {
        assert!(!check("swift"));
    }
}
