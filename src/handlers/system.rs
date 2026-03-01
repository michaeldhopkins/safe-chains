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

    safe! {
        brew_list: "brew list",
        brew_info: "brew info node",
        brew_version: "brew --version",
        brew_search: "brew search node",
        brew_deps: "brew deps node",
        brew_uses: "brew uses --installed openssl",
        brew_leaves: "brew leaves",
        brew_outdated: "brew outdated",
        brew_cat: "brew cat node",
        brew_desc: "brew desc node",
        brew_config: "brew config",
        brew_doctor: "brew doctor",
        brew_tap: "brew tap",
        brew_shellenv: "brew shellenv",
        brew_prefix: "brew --prefix",
        brew_prefix_formula: "brew --prefix libiconv",
        mise_ls: "mise ls",
        mise_list: "mise list ruby",
        mise_current: "mise current ruby",
        mise_which: "mise which ruby",
        mise_doctor: "mise doctor",
        mise_version: "mise --version",
        mise_settings_get: "mise settings get experimental",
        mise_env: "mise env",
        mise_config_ls: "mise config ls",
        mise_config_list: "mise config list",
        asdf_current: "asdf current ruby",
        asdf_which: "asdf which ruby",
        asdf_help: "asdf help",
        asdf_list: "asdf list ruby",
        asdf_version: "asdf --version",
        asdf_version_bare: "asdf version",
        asdf_info: "asdf info",
        asdf_plugin_list: "asdf plugin list",
        asdf_plugin_list_all: "asdf plugin list all",
        asdf_plugin_list_legacy: "asdf plugin-list",
        asdf_plugin_list_all_legacy: "asdf plugin-list-all",
        defaults_read: "defaults read com.apple.finder",
        defaults_read_type: "defaults read-type com.apple.finder ShowPathbar",
        defaults_domains: "defaults domains",
        defaults_find: "defaults find finder",
        defaults_export: "defaults export com.apple.finder -",
        sysctl_read: "sysctl kern.maxproc",
        sysctl_all: "sysctl -a",
        cmake_version: "cmake --version",
        cmake_system_information: "cmake --system-information",
        networksetup_listallhardwareports: "networksetup -listallhardwareports",
        networksetup_listallnetworkservices: "networksetup -listallnetworkservices",
        networksetup_getinfo: "networksetup -getinfo Wi-Fi",
        networksetup_getdnsservers: "networksetup -getdnsservers Wi-Fi",
        networksetup_version: "networksetup -version",
        networksetup_help: "networksetup -help",
        launchctl_list: "launchctl list",
        launchctl_print: "launchctl print system",
        launchctl_blame: "launchctl blame system/com.apple.Finder",
        launchctl_version: "launchctl version",
        diskutil_list: "diskutil list",
        diskutil_info: "diskutil info disk0",
        diskutil_apfs_list: "diskutil apfs list",
        diskutil_apfs_list_snapshots: "diskutil apfs listSnapshots disk1s1",
        security_find_identity: "security find-identity -v -p codesigning",
        security_find_certificate: "security find-certificate -a",
        security_list_keychains: "security list-keychains",
        security_verify_cert: "security verify-cert -c cert.pem",
        csrutil_status: "csrutil status",
        csrutil_report: "csrutil report",
        log_help: "log help",
        log_show: "log show --predicate 'process == \"Safari\"' --last 1h",
        log_stats: "log stats",
        log_stream: "log stream --level debug",
    }

    denied! {
        brew_install_denied: "brew install node",
        brew_uninstall_denied: "brew uninstall node",
        brew_services_denied: "brew services list",
        brew_upgrade_denied: "brew upgrade",
        mise_install_denied: "mise install ruby@3.4",
        mise_exec_denied: "mise exec -- ruby foo.rb",
        mise_use_denied: "mise use ruby@3.4",
        mise_config_set_denied: "mise config set key value",
        mise_config_bare_denied: "mise config",
        asdf_plugin_add_denied: "asdf plugin add ruby",
        asdf_install_denied: "asdf install ruby 3.4",
        defaults_write_denied: "defaults write com.apple.finder ShowPathbar -bool true",
        defaults_delete_denied: "defaults delete com.apple.finder",
        sysctl_write_denied: "sysctl -w kern.maxproc=2048",
        sysctl_write_long_denied: "sysctl --write kern.maxproc=2048",
        sysctl_assign_denied: "sysctl kern.maxproc=2048",
        cmake_build_denied: "cmake --build .",
        cmake_generate_denied: "cmake .",
        networksetup_setdnsservers_denied: "networksetup -setdnsservers Wi-Fi 8.8.8.8",
        networksetup_setairportpower_denied: "networksetup -setairportpower en0 on",
        networksetup_no_args_denied: "networksetup",
        launchctl_load_denied: "launchctl load /Library/LaunchDaemons/foo.plist",
        launchctl_start_denied: "launchctl start com.apple.Finder",
        launchctl_stop_denied: "launchctl stop com.apple.Finder",
        launchctl_no_args_denied: "launchctl",
        diskutil_apfs_bare_denied: "diskutil apfs",
        diskutil_erase_denied: "diskutil eraseDisk JHFS+ Untitled disk2",
        diskutil_mount_denied: "diskutil mount disk2s1",
        diskutil_unmount_denied: "diskutil unmount disk2s1",
        diskutil_apfs_delete_denied: "diskutil apfs deleteVolume disk1s2",
        diskutil_no_args_denied: "diskutil",
        security_add_denied: "security add-certificates cert.pem",
        security_delete_denied: "security delete-keychain test.keychain",
        security_no_args_denied: "security",
        csrutil_enable_denied: "csrutil enable",
        csrutil_disable_denied: "csrutil disable",
        csrutil_clear_denied: "csrutil clear",
        csrutil_no_args_denied: "csrutil",
        log_config_denied: "log config --mode level:debug",
        log_erase_denied: "log erase --all",
        log_no_args_denied: "log",
    }
}
