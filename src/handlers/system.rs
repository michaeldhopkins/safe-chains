use crate::parse::{Token, WordSet, has_flag};

static BREW_READ_ONLY: WordSet = WordSet::new(&[
    "--prefix", "--version", "casks", "cat", "config", "deps", "desc",
    "doctor", "formulae", "home", "info", "leaves", "list", "log",
    "outdated", "search", "shellenv", "tap", "uses",
]);

static MISE_READ_ONLY: WordSet =
    WordSet::new(&["--version", "current", "doctor", "env", "list", "ls", "which"]);

static MISE_MULTI: &[(&str, WordSet)] = &[
    ("config", WordSet::new(&["list", "ls"])),
    ("settings", WordSet::new(&["get"])),
];

static ASDF_READ_ONLY: WordSet =
    WordSet::new(&["--version", "current", "help", "info", "list", "version", "which"]);

static DEFAULTS_SAFE: WordSet =
    WordSet::new(&["domains", "export", "find", "read", "read-type"]);

pub fn is_safe_brew(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && BREW_READ_ONLY.contains(&tokens[1])
}

pub fn is_safe_mise(tokens: &[Token]) -> bool {
    super::is_safe_subcmd(tokens, &MISE_READ_ONLY, MISE_MULTI)
}

pub fn is_safe_asdf(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if ASDF_READ_ONLY.contains(&tokens[1]) {
        return true;
    }
    if tokens[1] == "plugin" {
        return tokens.get(2).is_some_and(|a| a == "list");
    }
    if tokens[1] == "plugin-list" || tokens[1] == "plugin-list-all" {
        return true;
    }
    false
}

pub fn is_safe_defaults(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && DEFAULTS_SAFE.contains(&tokens[1])
}

pub fn is_safe_sysctl(tokens: &[Token]) -> bool {
    !has_flag(tokens, Some("-w"), Some("--write"))
        && !tokens[1..].iter().any(|t| t.contains("="))
}

pub fn is_safe_cmake(tokens: &[Token]) -> bool {
    tokens.len() == 2 && (tokens[1] == "--version" || tokens[1] == "--system-information")
}

static SECURITY_READ_ONLY: WordSet = WordSet::new(&[
    "cms", "dump-keychain", "dump-trust-settings", "find-certificate",
    "find-generic-password", "find-identity", "find-internet-password",
    "list-keychains", "show-keychain-info", "smartcard", "verify-cert",
]);

pub fn is_safe_security(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && SECURITY_READ_ONLY.contains(&tokens[1])
}

static CSRUTIL_READ_ONLY: WordSet =
    WordSet::new(&["authenticated-root", "report", "status"]);

pub fn is_safe_csrutil(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && CSRUTIL_READ_ONLY.contains(&tokens[1])
}

static DISKUTIL_READ_ONLY: WordSet =
    WordSet::new(&["activity", "info", "list", "listFilesystems"]);

static DISKUTIL_APFS_READ_ONLY: WordSet =
    WordSet::new(&["list", "listCryptoUsers", "listSnapshots", "listVolumeGroups"]);

pub fn is_safe_diskutil(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if tokens[1] == "apfs" {
        return tokens.get(2).is_some_and(|a| DISKUTIL_APFS_READ_ONLY.contains(a));
    }
    DISKUTIL_READ_ONLY.contains(&tokens[1])
}

static LAUNCHCTL_READ_ONLY: WordSet = WordSet::new(&[
    "blame", "dumpstate", "error", "examine", "help", "hostinfo",
    "list", "print", "print-cache", "print-disabled", "resolveport", "version",
]);

pub fn is_safe_launchctl(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && LAUNCHCTL_READ_ONLY.contains(&tokens[1])
}

pub fn is_safe_networksetup(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    tokens[1].starts_with("-list")
        || tokens[1].starts_with("-get")
        || tokens[1].starts_with("-show")
        || tokens[1].starts_with("-print")
        || tokens[1] == "-version"
        || tokens[1] == "-help"
}

static LOG_READ_ONLY: WordSet =
    WordSet::new(&["help", "show", "stats", "stream"]);

pub fn is_safe_log(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && LOG_READ_ONLY.contains(&tokens[1])
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, doc};
    vec![
        CommandDoc::wordset("brew", &BREW_READ_ONLY),
        CommandDoc::wordset_multi("mise", &MISE_READ_ONLY, MISE_MULTI),
        CommandDoc::handler("asdf",
            doc(&ASDF_READ_ONLY)
                .multi_word(&[("plugin", WordSet::new(&["list"]))])
                .subcommand("plugin-list")
                .subcommand("plugin-list-all")
                .build()),
        CommandDoc::wordset("defaults", &DEFAULTS_SAFE),
        CommandDoc::handler("sysctl",
            "Safe unless -w/--write flag or key=value assignment syntax."),
        CommandDoc::handler("cmake",
            "Allowed: --version, --system-information (single argument only)."),
        CommandDoc::wordset("security", &SECURITY_READ_ONLY),
        CommandDoc::wordset("csrutil", &CSRUTIL_READ_ONLY),
        CommandDoc::handler("diskutil",
            doc(&DISKUTIL_READ_ONLY)
                .multi_word(&[("apfs", DISKUTIL_APFS_READ_ONLY)])
                .build()),
        CommandDoc::wordset("launchctl", &LAUNCHCTL_READ_ONLY),
        CommandDoc::handler("networksetup",
            "Allowed: subcommands starting with -list, -get, -show, -print, plus -version and -help."),
        CommandDoc::wordset("log", &LOG_READ_ONLY),
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
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
    fn brew_prefix() {
        assert!(check("brew --prefix"));
    }

    #[test]
    fn brew_prefix_formula() {
        assert!(check("brew --prefix libiconv"));
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
    fn mise_env() {
        assert!(check("mise env"));
    }

    #[test]
    fn mise_config_ls() {
        assert!(check("mise config ls"));
    }

    #[test]
    fn mise_config_list() {
        assert!(check("mise config list"));
    }

    #[test]
    fn mise_config_set_denied() {
        assert!(!check("mise config set key value"));
    }

    #[test]
    fn mise_config_bare_denied() {
        assert!(!check("mise config"));
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
    fn asdf_version_bare() {
        assert!(check("asdf version"));
    }

    #[test]
    fn asdf_info() {
        assert!(check("asdf info"));
    }

    #[test]
    fn asdf_plugin_list() {
        assert!(check("asdf plugin list"));
    }

    #[test]
    fn asdf_plugin_list_all() {
        assert!(check("asdf plugin list all"));
    }

    #[test]
    fn asdf_plugin_list_legacy() {
        assert!(check("asdf plugin-list"));
    }

    #[test]
    fn asdf_plugin_list_all_legacy() {
        assert!(check("asdf plugin-list-all"));
    }

    #[test]
    fn asdf_plugin_add_denied() {
        assert!(!check("asdf plugin add ruby"));
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

    #[test]
    fn networksetup_listallhardwareports() {
        assert!(check("networksetup -listallhardwareports"));
    }

    #[test]
    fn networksetup_listallnetworkservices() {
        assert!(check("networksetup -listallnetworkservices"));
    }

    #[test]
    fn networksetup_getinfo() {
        assert!(check("networksetup -getinfo Wi-Fi"));
    }

    #[test]
    fn networksetup_getdnsservers() {
        assert!(check("networksetup -getdnsservers Wi-Fi"));
    }

    #[test]
    fn networksetup_version() {
        assert!(check("networksetup -version"));
    }

    #[test]
    fn networksetup_help() {
        assert!(check("networksetup -help"));
    }

    #[test]
    fn networksetup_setdnsservers_denied() {
        assert!(!check("networksetup -setdnsservers Wi-Fi 8.8.8.8"));
    }

    #[test]
    fn networksetup_setairportpower_denied() {
        assert!(!check("networksetup -setairportpower en0 on"));
    }

    #[test]
    fn networksetup_no_args_denied() {
        assert!(!check("networksetup"));
    }

    #[test]
    fn launchctl_list() {
        assert!(check("launchctl list"));
    }

    #[test]
    fn launchctl_print() {
        assert!(check("launchctl print system"));
    }

    #[test]
    fn launchctl_blame() {
        assert!(check("launchctl blame system/com.apple.Finder"));
    }

    #[test]
    fn launchctl_version() {
        assert!(check("launchctl version"));
    }

    #[test]
    fn launchctl_load_denied() {
        assert!(!check("launchctl load /Library/LaunchDaemons/foo.plist"));
    }

    #[test]
    fn launchctl_start_denied() {
        assert!(!check("launchctl start com.apple.Finder"));
    }

    #[test]
    fn launchctl_stop_denied() {
        assert!(!check("launchctl stop com.apple.Finder"));
    }

    #[test]
    fn launchctl_no_args_denied() {
        assert!(!check("launchctl"));
    }

    #[test]
    fn diskutil_list() {
        assert!(check("diskutil list"));
    }

    #[test]
    fn diskutil_info() {
        assert!(check("diskutil info disk0"));
    }

    #[test]
    fn diskutil_apfs_list() {
        assert!(check("diskutil apfs list"));
    }

    #[test]
    fn diskutil_apfs_list_snapshots() {
        assert!(check("diskutil apfs listSnapshots disk1s1"));
    }

    #[test]
    fn diskutil_apfs_bare_denied() {
        assert!(!check("diskutil apfs"));
    }

    #[test]
    fn diskutil_erase_denied() {
        assert!(!check("diskutil eraseDisk JHFS+ Untitled disk2"));
    }

    #[test]
    fn diskutil_mount_denied() {
        assert!(!check("diskutil mount disk2s1"));
    }

    #[test]
    fn diskutil_unmount_denied() {
        assert!(!check("diskutil unmount disk2s1"));
    }

    #[test]
    fn diskutil_apfs_delete_denied() {
        assert!(!check("diskutil apfs deleteVolume disk1s2"));
    }

    #[test]
    fn diskutil_no_args_denied() {
        assert!(!check("diskutil"));
    }

    #[test]
    fn security_find_identity() {
        assert!(check("security find-identity -v -p codesigning"));
    }

    #[test]
    fn security_find_certificate() {
        assert!(check("security find-certificate -a"));
    }

    #[test]
    fn security_list_keychains() {
        assert!(check("security list-keychains"));
    }

    #[test]
    fn security_verify_cert() {
        assert!(check("security verify-cert -c cert.pem"));
    }

    #[test]
    fn security_add_denied() {
        assert!(!check("security add-certificates cert.pem"));
    }

    #[test]
    fn security_delete_denied() {
        assert!(!check("security delete-keychain test.keychain"));
    }

    #[test]
    fn security_no_args_denied() {
        assert!(!check("security"));
    }

    #[test]
    fn csrutil_status() {
        assert!(check("csrutil status"));
    }

    #[test]
    fn csrutil_report() {
        assert!(check("csrutil report"));
    }

    #[test]
    fn csrutil_enable_denied() {
        assert!(!check("csrutil enable"));
    }

    #[test]
    fn csrutil_disable_denied() {
        assert!(!check("csrutil disable"));
    }

    #[test]
    fn csrutil_clear_denied() {
        assert!(!check("csrutil clear"));
    }

    #[test]
    fn csrutil_no_args_denied() {
        assert!(!check("csrutil"));
    }

    #[test]
    fn log_help() {
        assert!(check("log help"));
    }

    #[test]
    fn log_show() {
        assert!(check("log show --predicate 'process == \"Safari\"' --last 1h"));
    }

    #[test]
    fn log_stats() {
        assert!(check("log stats"));
    }

    #[test]
    fn log_stream() {
        assert!(check("log stream --level debug"));
    }

    #[test]
    fn log_config_denied() {
        assert!(!check("log config --mode level:debug"));
    }

    #[test]
    fn log_erase_denied() {
        assert!(!check("log erase --all"));
    }

    #[test]
    fn log_no_args_denied() {
        assert!(!check("log"));
    }
}
