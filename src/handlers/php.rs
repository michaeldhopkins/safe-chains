use std::collections::HashSet;
use std::sync::LazyLock;

use crate::parse::Token;

static COMPOSER_SAFE: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "show",
        "info",
        "diagnose",
        "outdated",
        "licenses",
        "check-platform-reqs",
        "suggests",
        "fund",
        "audit",
        "--version",
        "about",
        "help",
    ])
});

pub fn is_safe_composer(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && COMPOSER_SAFE.contains(tokens[1].as_str())
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocKind};
    vec![CommandDoc {
        name: "composer",
        kind: DocKind::Handler,
        description: "Allowed: show, info, diagnose, outdated, licenses, check-platform-reqs, suggests, fund, audit, --version, about, help.",
    }]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    #[test]
    fn composer_show() {
        assert!(check("composer show"));
    }

    #[test]
    fn composer_show_package() {
        assert!(check("composer show laravel/framework"));
    }

    #[test]
    fn composer_info() {
        assert!(check("composer info"));
    }

    #[test]
    fn composer_diagnose() {
        assert!(check("composer diagnose"));
    }

    #[test]
    fn composer_outdated() {
        assert!(check("composer outdated"));
    }

    #[test]
    fn composer_licenses() {
        assert!(check("composer licenses"));
    }

    #[test]
    fn composer_check_platform_reqs() {
        assert!(check("composer check-platform-reqs"));
    }

    #[test]
    fn composer_suggests() {
        assert!(check("composer suggests"));
    }

    #[test]
    fn composer_fund() {
        assert!(check("composer fund"));
    }

    #[test]
    fn composer_audit() {
        assert!(check("composer audit"));
    }

    #[test]
    fn composer_version() {
        assert!(check("composer --version"));
    }

    #[test]
    fn composer_about() {
        assert!(check("composer about"));
    }

    #[test]
    fn composer_help() {
        assert!(check("composer help"));
    }

    #[test]
    fn composer_install_denied() {
        assert!(!check("composer install"));
    }

    #[test]
    fn composer_update_denied() {
        assert!(!check("composer update"));
    }

    #[test]
    fn composer_require_denied() {
        assert!(!check("composer require laravel/framework"));
    }

    #[test]
    fn composer_remove_denied() {
        assert!(!check("composer remove laravel/framework"));
    }

    #[test]
    fn composer_run_script_denied() {
        assert!(!check("composer run-script test"));
    }

    #[test]
    fn bare_composer_denied() {
        assert!(!check("composer"));
    }
}
