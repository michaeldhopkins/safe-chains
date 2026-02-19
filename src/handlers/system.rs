use std::collections::HashSet;
use std::sync::LazyLock;

use crate::parse::has_flag;

static BREW_READ_ONLY: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "list", "info", "--version", "search", "deps", "uses", "leaves", "outdated", "cat",
        "desc", "home", "formulae", "casks", "config", "doctor", "log", "tap", "shellenv",
    ])
});

static MISE_READ_ONLY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["ls", "list", "current", "which", "doctor", "--version"]));

static MISE_MULTI: LazyLock<Vec<(&'static str, HashSet<&'static str>)>> =
    LazyLock::new(|| vec![("settings", HashSet::from(["get"]))]);

static ASDF_READ_ONLY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["current", "which", "help", "list", "--version"]));

static DEFAULTS_SAFE: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["read", "read-type", "domains", "find", "export"]));

static XCODEBUILD_SAFE: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "-version",
        "-showsdks",
        "-showBuildSettings",
        "-showdestinations",
        "-list",
    ])
});

pub fn is_safe_brew(tokens: &[String]) -> bool {
    tokens.len() >= 2 && BREW_READ_ONLY.contains(tokens[1].as_str())
}

pub fn is_safe_mise(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if MISE_READ_ONLY.contains(tokens[1].as_str()) {
        return true;
    }
    for (prefix, actions) in MISE_MULTI.iter() {
        if tokens[1] == *prefix {
            return tokens.get(2).is_some_and(|a| actions.contains(a.as_str()));
        }
    }
    false
}

pub fn is_safe_asdf(tokens: &[String]) -> bool {
    tokens.len() >= 2 && ASDF_READ_ONLY.contains(tokens[1].as_str())
}

pub fn is_safe_defaults(tokens: &[String]) -> bool {
    tokens.len() >= 2 && DEFAULTS_SAFE.contains(tokens[1].as_str())
}

pub fn is_safe_sysctl(tokens: &[String]) -> bool {
    !has_flag(tokens, "-w", Some("--write"))
        && !tokens[1..].iter().any(|t| t.contains('='))
}

pub fn is_safe_xcodebuild(tokens: &[String]) -> bool {
    tokens.len() >= 2 && XCODEBUILD_SAFE.contains(tokens[1].as_str())
}

pub fn is_safe_cmake(tokens: &[String]) -> bool {
    tokens.len() == 2 && (tokens[1] == "--version" || tokens[1] == "--system-information")
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocKind};
    vec![
        CommandDoc {
            name: "brew",
            kind: DocKind::Handler,
            description: "Allowed: list, info, --version, search, deps, uses, leaves, outdated, cat, desc, home, formulae, casks, config, doctor, log, tap, shellenv.",
        },
        CommandDoc {
            name: "mise",
            kind: DocKind::Handler,
            description: "Allowed: ls, list, current, which, doctor, --version. Multi-word: settings get.",
        },
        CommandDoc {
            name: "asdf",
            kind: DocKind::Handler,
            description: "Allowed: current, which, help, list, --version.",
        },
        CommandDoc {
            name: "defaults",
            kind: DocKind::Handler,
            description: "Allowed: read, read-type, domains, find, export.",
        },
        CommandDoc {
            name: "sysctl",
            kind: DocKind::Handler,
            description: "Safe unless -w/--write flag or key=value assignment syntax.",
        },
        CommandDoc {
            name: "xcodebuild",
            kind: DocKind::Handler,
            description: "Allowed: -version, -showsdks, -showBuildSettings, -showdestinations, -list.",
        },
        CommandDoc {
            name: "cmake",
            kind: DocKind::Handler,
            description: "Allowed: --version, --system-information (single argument only).",
        },
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe;

    fn check(cmd: &str) -> bool {
        is_safe(cmd)
    }

    #[test]
    fn brew_list() {
        assert!(check("brew list"));
    }

    #[test]
    fn brew_info() {
        assert!(check("brew info node"));
    }

    #[test]
    fn brew_version() {
        assert!(check("brew --version"));
    }

    #[test]
    fn brew_search() {
        assert!(check("brew search node"));
    }

    #[test]
    fn brew_deps() {
        assert!(check("brew deps node"));
    }

    #[test]
    fn brew_uses() {
        assert!(check("brew uses --installed openssl"));
    }

    #[test]
    fn brew_leaves() {
        assert!(check("brew leaves"));
    }

    #[test]
    fn brew_outdated() {
        assert!(check("brew outdated"));
    }

    #[test]
    fn brew_cat() {
        assert!(check("brew cat node"));
    }

    #[test]
    fn brew_desc() {
        assert!(check("brew desc node"));
    }

    #[test]
    fn brew_config() {
        assert!(check("brew config"));
    }

    #[test]
    fn brew_doctor() {
        assert!(check("brew doctor"));
    }

    #[test]
    fn brew_tap() {
        assert!(check("brew tap"));
    }

    #[test]
    fn brew_shellenv() {
        assert!(check("brew shellenv"));
    }

    #[test]
    fn brew_install_denied() {
        assert!(!check("brew install node"));
    }

    #[test]
    fn brew_uninstall_denied() {
        assert!(!check("brew uninstall node"));
    }

    #[test]
    fn brew_services_denied() {
        assert!(!check("brew services list"));
    }

    #[test]
    fn brew_upgrade_denied() {
        assert!(!check("brew upgrade"));
    }

    #[test]
    fn mise_ls() {
        assert!(check("mise ls"));
    }

    #[test]
    fn mise_list() {
        assert!(check("mise list ruby"));
    }

    #[test]
    fn mise_current() {
        assert!(check("mise current ruby"));
    }

    #[test]
    fn mise_which() {
        assert!(check("mise which ruby"));
    }

    #[test]
    fn mise_doctor() {
        assert!(check("mise doctor"));
    }

    #[test]
    fn mise_version() {
        assert!(check("mise --version"));
    }

    #[test]
    fn mise_settings_get() {
        assert!(check("mise settings get experimental"));
    }

    #[test]
    fn mise_install_denied() {
        assert!(!check("mise install ruby@3.4"));
    }

    #[test]
    fn mise_exec_denied() {
        assert!(!check("mise exec -- ruby foo.rb"));
    }

    #[test]
    fn mise_use_denied() {
        assert!(!check("mise use ruby@3.4"));
    }

    #[test]
    fn asdf_current() {
        assert!(check("asdf current ruby"));
    }

    #[test]
    fn asdf_which() {
        assert!(check("asdf which ruby"));
    }

    #[test]
    fn asdf_help() {
        assert!(check("asdf help"));
    }

    #[test]
    fn asdf_list() {
        assert!(check("asdf list ruby"));
    }

    #[test]
    fn asdf_version() {
        assert!(check("asdf --version"));
    }

    #[test]
    fn asdf_install_denied() {
        assert!(!check("asdf install ruby 3.4"));
    }

    #[test]
    fn defaults_read() {
        assert!(check("defaults read com.apple.finder"));
    }

    #[test]
    fn defaults_read_type() {
        assert!(check("defaults read-type com.apple.finder ShowPathbar"));
    }

    #[test]
    fn defaults_domains() {
        assert!(check("defaults domains"));
    }

    #[test]
    fn defaults_find() {
        assert!(check("defaults find finder"));
    }

    #[test]
    fn defaults_export() {
        assert!(check("defaults export com.apple.finder -"));
    }

    #[test]
    fn defaults_write_denied() {
        assert!(!check("defaults write com.apple.finder ShowPathbar -bool true"));
    }

    #[test]
    fn defaults_delete_denied() {
        assert!(!check("defaults delete com.apple.finder"));
    }

    #[test]
    fn sysctl_read() {
        assert!(check("sysctl kern.maxproc"));
    }

    #[test]
    fn sysctl_all() {
        assert!(check("sysctl -a"));
    }

    #[test]
    fn sysctl_write_denied() {
        assert!(!check("sysctl -w kern.maxproc=2048"));
    }

    #[test]
    fn sysctl_write_long_denied() {
        assert!(!check("sysctl --write kern.maxproc=2048"));
    }

    #[test]
    fn sysctl_assign_denied() {
        assert!(!check("sysctl kern.maxproc=2048"));
    }

    #[test]
    fn xcodebuild_version() {
        assert!(check("xcodebuild -version"));
    }

    #[test]
    fn xcodebuild_showsdks() {
        assert!(check("xcodebuild -showsdks"));
    }

    #[test]
    fn xcodebuild_show_build_settings() {
        assert!(check("xcodebuild -showBuildSettings"));
    }

    #[test]
    fn xcodebuild_list() {
        assert!(check("xcodebuild -list"));
    }

    #[test]
    fn xcodebuild_build_denied() {
        assert!(!check("xcodebuild build"));
    }

    #[test]
    fn xcodebuild_clean_denied() {
        assert!(!check("xcodebuild clean"));
    }

    #[test]
    fn cmake_version() {
        assert!(check("cmake --version"));
    }

    #[test]
    fn cmake_system_information() {
        assert!(check("cmake --system-information"));
    }

    #[test]
    fn cmake_build_denied() {
        assert!(!check("cmake --build ."));
    }

    #[test]
    fn cmake_generate_denied() {
        assert!(!check("cmake ."));
    }
}
