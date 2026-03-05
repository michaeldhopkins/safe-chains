use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy};

static BREW_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--cask", "--formula", "--full-name", "--multiple",
        "--pinned", "--versions",
    ]),
    standalone_short: b"1lrt",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static BREW_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--analytics", "--cask", "--formula", "--installed", "--json",
    ]),
    standalone_short: b"v",
    valued: WordSet::new(&["--days"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static BREW_SEARCH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--cask", "--closed", "--debian", "--desc", "--fedora",
        "--fink", "--formula", "--macports", "--open",
        "--opensuse", "--pull-request", "--repology", "--ubuntu",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static BREW_DEPS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--1", "--annotate", "--cask", "--direct", "--for-each",
        "--formula", "--full-name", "--graph", "--include-build",
        "--include-optional", "--include-test", "--installed", "--missing",
        "--skip-recommended", "--tree", "--union",
    ]),
    standalone_short: b"n",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static BREW_USES_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--cask", "--formula", "--include-build", "--include-optional",
        "--include-test", "--installed", "--missing",
        "--recursive", "--skip-recommended",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static BREW_OUTDATED_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--cask", "--fetch-HEAD", "--formula", "--greedy",
        "--greedy-auto-updates", "--greedy-latest", "--json",
    ]),
    standalone_short: b"dqv",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static BREW_DESC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--cask", "--description", "--eval-all", "--formula",
        "--name", "--search",
    ]),
    standalone_short: b"dns",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static BREW_LOG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--cask", "--formula", "--oneline",
    ]),
    standalone_short: b"1",
    valued: WordSet::new(&["--max-count"]),
    valued_short: b"n",
    bare: false,
    max_positional: None,
};

static BREW_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"qv",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_brew(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let policy = match tokens[1].as_str() {
        "list" | "ls" => &BREW_LIST_POLICY,
        "info" | "abv" => &BREW_INFO_POLICY,
        "search" => &BREW_SEARCH_POLICY,
        "deps" => &BREW_DEPS_POLICY,
        "uses" => &BREW_USES_POLICY,
        "outdated" => &BREW_OUTDATED_POLICY,
        "desc" => &BREW_DESC_POLICY,
        "log" => &BREW_LOG_POLICY,
        "cat" | "casks" | "config" | "doctor" | "formulae"
        | "home" | "leaves" | "shellenv" | "tap" => &BREW_SIMPLE_POLICY,
        "--prefix" => &BREW_SIMPLE_POLICY,
        _ => return false,
    };
    policy::check(&tokens[1..], policy)
}

static MISE_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"qv",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static MISE_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--current", "--installed", "--json", "--missing",
        "--no-header", "--prefix",
    ]),
    standalone_short: b"ciJm",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static MISE_ENV_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json"]),
    standalone_short: b"J",
    valued: WordSet::new(&["--shell"]),
    valued_short: b"s",
    bare: true,
    max_positional: None,
};

pub fn is_safe_mise(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    match tokens[1].as_str() {
        "list" | "ls" => policy::check(&tokens[1..], &MISE_LIST_POLICY),
        "current" | "which" | "doctor" => policy::check(&tokens[1..], &MISE_SIMPLE_POLICY),
        "env" => policy::check(&tokens[1..], &MISE_ENV_POLICY),
        "config" => {
            tokens.get(2).is_some_and(|a| a == "list" || a == "ls")
                && policy::check(&tokens[2..], &MISE_SIMPLE_POLICY)
        }
        "settings" => {
            tokens.get(2).is_some_and(|a| a == "get")
                && policy::check(&tokens[2..], &MISE_SIMPLE_POLICY)
        }
        _ => false,
    }
}

static ASDF_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_asdf(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    static ASDF_SAFE: WordSet = WordSet::new(&[
        "current", "help", "info", "list", "version",
    ]);
    match tokens[1].as_str() {
        s if ASDF_SAFE.contains(s) => policy::check(&tokens[1..], &ASDF_SIMPLE_POLICY),
        "plugin" => {
            tokens.get(2).is_some_and(|a| a == "list")
                && policy::check(&tokens[2..], &ASDF_SIMPLE_POLICY)
        }
        "plugin-list" | "plugin-list-all" => {
            policy::check(&tokens[1..], &ASDF_SIMPLE_POLICY)
        }
        "which" => policy::check(&tokens[1..], &ASDF_SIMPLE_POLICY),
        _ => false,
    }
}

static DEFAULTS_READ_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-g", "-globalDomain"]),
    standalone_short: b"",
    valued: WordSet::new(&["-app"]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static DEFAULTS_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_defaults(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    match tokens[1].as_str() {
        "read" | "read-type" | "export" | "find" => {
            policy::check(&tokens[1..], &DEFAULTS_READ_POLICY)
        }
        "domains" => policy::check(&tokens[1..], &DEFAULTS_SIMPLE_POLICY),
        _ => false,
    }
}

static PMSET_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_pmset(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if tokens[1] != "-g" {
        return false;
    }
    policy::check(&tokens[2..], &PMSET_POLICY)
}

static SYSCTL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-A", "-N", "-X", "-a", "-b", "-d", "-e", "-h",
        "-l", "-n", "-o", "-q", "-x",
    ]),
    standalone_short: b"ANXabdehlnoqx",
    valued: WordSet::new(&["-B", "-r"]),
    valued_short: b"Br",
    bare: false,
    max_positional: None,
};

pub fn is_safe_sysctl(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if tokens[1..].iter().any(|t| t.contains("=")) {
        return false;
    }
    policy::check(tokens, &SYSCTL_POLICY)
}

pub fn is_safe_cmake(tokens: &[Token]) -> bool {
    tokens.len() == 2 && (tokens[1] == "--version" || tokens[1] == "--system-information")
}

static SECURITY_FIND_CERT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-Z", "-a", "-p"]),
    standalone_short: b"Zap",
    valued: WordSet::new(&["-c", "-e"]),
    valued_short: b"ce",
    bare: false,
    max_positional: None,
};

static SECURITY_FIND_IDENTITY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-v"]),
    standalone_short: b"v",
    valued: WordSet::new(&["-p", "-s"]),
    valued_short: b"ps",
    bare: true,
    max_positional: None,
};

static SECURITY_FIND_PASSWORD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "-D", "-a", "-c", "-d", "-j", "-l", "-r", "-s",
        "-t",
    ]),
    valued_short: b"Dacdjlrst",
    bare: false,
    max_positional: None,
};

static SECURITY_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-d"]),
    standalone_short: b"d",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static SECURITY_DUMP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static SECURITY_VERIFY_CERT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-L", "-l", "-q"]),
    standalone_short: b"Llq",
    valued: WordSet::new(&["-c", "-k", "-n", "-p", "-r"]),
    valued_short: b"cknpr",
    bare: false,
    max_positional: None,
};

static SECURITY_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_security(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let policy = match tokens[1].as_str() {
        "find-certificate" => &SECURITY_FIND_CERT_POLICY,
        "find-identity" => &SECURITY_FIND_IDENTITY_POLICY,
        "find-generic-password" | "find-internet-password" => &SECURITY_FIND_PASSWORD_POLICY,
        "list-keychains" => &SECURITY_LIST_POLICY,
        "dump-keychain" | "dump-trust-settings" => &SECURITY_DUMP_POLICY,
        "verify-cert" => &SECURITY_VERIFY_CERT_POLICY,
        "cms" | "show-keychain-info" | "smartcard" => &SECURITY_SIMPLE_POLICY,
        _ => return false,
    };
    policy::check(&tokens[1..], policy)
}

static CSRUTIL_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_csrutil(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    static CSRUTIL_SAFE: WordSet =
        WordSet::new(&["authenticated-root", "report", "status"]);
    if !CSRUTIL_SAFE.contains(&tokens[1]) {
        return false;
    }
    policy::check(&tokens[1..], &CSRUTIL_SIMPLE_POLICY)
}

static DISKUTIL_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static DISKUTIL_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-plist"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static DISKUTIL_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-all", "-plist"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

pub fn is_safe_diskutil(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    match tokens[1].as_str() {
        "list" | "listFilesystems" => policy::check(&tokens[1..], &DISKUTIL_LIST_POLICY),
        "info" => policy::check(&tokens[1..], &DISKUTIL_INFO_POLICY),
        "activity" => policy::check(&tokens[1..], &DISKUTIL_SIMPLE_POLICY),
        "apfs" => {
            static APFS_SAFE: WordSet =
                WordSet::new(&["list", "listCryptoUsers", "listSnapshots", "listVolumeGroups"]);
            tokens.get(2).is_some_and(|a| APFS_SAFE.contains(a))
                && policy::check(&tokens[2..], &DISKUTIL_SIMPLE_POLICY)
        }
        _ => false,
    }
}

static LAUNCHCTL_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_launchctl(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    static LAUNCHCTL_SAFE: WordSet = WordSet::new(&[
        "blame", "dumpstate", "error", "examine", "help", "hostinfo",
        "list", "print", "print-cache", "print-disabled", "resolveport", "version",
    ]);
    if !LAUNCHCTL_SAFE.contains(&tokens[1]) {
        return false;
    }
    policy::check(&tokens[1..], &LAUNCHCTL_SIMPLE_POLICY)
}

pub fn is_safe_networksetup(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let sub = tokens[1].as_str();
    if !(sub.starts_with("-list")
        || sub.starts_with("-get")
        || sub.starts_with("-show")
        || sub.starts_with("-print")
        || sub == "-version"
        || sub == "-help")
    {
        return false;
    }
    true
}

static LOG_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--backtrace", "--debug", "--info", "--loss", "--mach-continuous-time",
        "--no-pager", "--signpost",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "--color", "--end", "--last", "--predicate",
        "--process", "--source", "--start", "--style", "--type",
    ]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static LOG_STREAM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--backtrace", "--debug", "--info", "--loss",
        "--mach-continuous-time", "--signpost",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "--color", "--level", "--predicate", "--process",
        "--source", "--style", "--timeout", "--type",
    ]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static LOG_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_log(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let policy = match tokens[1].as_str() {
        "show" => &LOG_SHOW_POLICY,
        "stream" => &LOG_STREAM_POLICY,
        "help" | "stats" => &LOG_SIMPLE_POLICY,
        _ => return false,
    };
    policy::check(&tokens[1..], policy)
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "brew" => Some(is_safe_brew(tokens)),
        "mise" => Some(is_safe_mise(tokens)),
        "asdf" => Some(is_safe_asdf(tokens)),
        "defaults" => Some(is_safe_defaults(tokens)),
        "pmset" => Some(is_safe_pmset(tokens)),
        "sysctl" => Some(is_safe_sysctl(tokens)),
        "cmake" => Some(is_safe_cmake(tokens)),
        "security" => Some(is_safe_security(tokens)),
        "csrutil" => Some(is_safe_csrutil(tokens)),
        "diskutil" => Some(is_safe_diskutil(tokens)),
        "launchctl" => Some(is_safe_launchctl(tokens)),
        "networksetup" => Some(is_safe_networksetup(tokens)),
        "log" => Some(is_safe_log(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("brew",
            "Subcommands: cat, casks, config, deps, desc, doctor, formulae, home, info, \
             leaves, list, log, outdated, search, shellenv, tap, uses. \
            "),
        CommandDoc::handler("mise",
            "Subcommands: current, doctor, env, list/ls, which. \
             Multi-level: config list/ls, settings get. \
            "),
        CommandDoc::handler("asdf",
            "Subcommands: current, help, info, list, version, which. \
             Multi-level: plugin list. Also: plugin-list, plugin-list-all. \
"),
        CommandDoc::handler("defaults",
            "Subcommands: domains, export, find, read, read-type. \
            "),
        CommandDoc::handler("pmset",
            "Allowed: -g (get/display settings only)."),
        CommandDoc::handler("sysctl",
            "Read-only usage."),
        CommandDoc::handler("cmake",
            "Allowed: --version, --system-information (single argument only)."),
        CommandDoc::handler("security",
            "Subcommands: cms, dump-keychain, dump-trust-settings, find-certificate, \
             find-generic-password, find-identity, find-internet-password, \
             list-keychains, show-keychain-info, smartcard, verify-cert. \
            "),
        CommandDoc::handler("csrutil",
            "Subcommands: authenticated-root, report, status."),
        CommandDoc::handler("diskutil",
            "Subcommands: activity, info, list, listFilesystems. \
             Multi-level: apfs list/listCryptoUsers/listSnapshots/listVolumeGroups. \
            "),
        CommandDoc::handler("launchctl",
            "Subcommands: blame, dumpstate, error, examine, help, hostinfo, \
             list, print, print-cache, print-disabled, resolveport, version. \
"),
        CommandDoc::handler("networksetup",
            "Allowed: subcommands starting with -list, -get, -show, -print, \
             plus -version and -help."),
        CommandDoc::handler("log",
            "Subcommands: help, show, stats, stream. \
"),
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
        brew_list_formula: "brew list --formula",
        brew_list_cask: "brew list --cask",
        brew_list_versions: "brew list --versions",
        brew_list_full_name: "brew list --full-name",
        brew_info: "brew info node",
        brew_info_json: "brew info --json node",
        brew_info_installed: "brew info --installed",
        brew_version: "brew --version",
        brew_search: "brew search node",
        brew_search_desc: "brew search --desc node",
        brew_search_cask: "brew search --cask node",
        brew_deps: "brew deps node",
        brew_deps_tree: "brew deps --tree node",
        brew_deps_installed: "brew deps --installed",
        brew_deps_annotate: "brew deps --annotate node",
        brew_uses: "brew uses --installed openssl",
        brew_uses_recursive: "brew uses --recursive openssl",
        brew_leaves: "brew leaves",
        brew_outdated: "brew outdated",
        brew_outdated_json: "brew outdated --json",
        brew_outdated_greedy: "brew outdated --greedy",
        brew_cat: "brew cat node",
        brew_desc: "brew desc node",
        brew_desc_search: "brew desc --search node",
        brew_config: "brew config",
        brew_doctor: "brew doctor",
        brew_tap: "brew tap",
        brew_shellenv: "brew shellenv",
        brew_prefix: "brew --prefix",
        brew_prefix_formula: "brew --prefix libiconv",
        brew_home: "brew home node",
        brew_formulae: "brew formulae",
        brew_casks: "brew casks",
        brew_log: "brew log node",
        brew_log_oneline: "brew log --oneline node",
        mise_ls: "mise ls",
        mise_ls_current: "mise ls --current",
        mise_ls_json: "mise ls --json",
        mise_list: "mise list ruby",
        mise_current: "mise current ruby",
        mise_which: "mise which ruby",
        mise_doctor: "mise doctor",
        mise_version: "mise --version",
        mise_settings_get: "mise settings get experimental",
        mise_env: "mise env",
        mise_env_json: "mise env --json",
        mise_env_shell: "mise env --shell bash",
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
        defaults_read_global: "defaults read -g",
        pmset_get: "pmset -g",
        pmset_get_assertions: "pmset -g assertions",
        pmset_get_batt: "pmset -g batt",
        sysctl_read: "sysctl kern.maxproc",
        sysctl_all: "sysctl -a",
        sysctl_names: "sysctl -N -a",
        cmake_version: "cmake --version",
        cmake_system_information: "cmake --system-information",
        security_find_identity: "security find-identity -v -p codesigning",
        security_find_certificate: "security find-certificate -a",
        security_list_keychains: "security list-keychains",
        security_verify_cert: "security verify-cert -c cert.pem",
        security_dump_keychain: "security dump-keychain",
        security_dump_trust: "security dump-trust-settings",
        csrutil_status: "csrutil status",
        csrutil_report: "csrutil report",
        csrutil_authenticated_root: "csrutil authenticated-root",
        launchctl_list: "launchctl list",
        launchctl_print: "launchctl print system",
        launchctl_blame: "launchctl blame system/com.apple.Finder",
        launchctl_version: "launchctl version",
        launchctl_help: "launchctl help",
        launchctl_hostinfo: "launchctl hostinfo",
        diskutil_list: "diskutil list",
        diskutil_list_plist: "diskutil list -plist",
        diskutil_info: "diskutil info disk0",
        diskutil_info_plist: "diskutil info -plist disk0",
        diskutil_info_all: "diskutil info -all",
        diskutil_activity: "diskutil activity",
        diskutil_list_filesystems: "diskutil listFilesystems",
        diskutil_apfs_list: "diskutil apfs list",
        diskutil_apfs_list_snapshots: "diskutil apfs listSnapshots disk1s1",
        diskutil_apfs_list_crypto: "diskutil apfs listCryptoUsers disk1s1",
        diskutil_apfs_list_volume_groups: "diskutil apfs listVolumeGroups",
        networksetup_listallhardwareports: "networksetup -listallhardwareports",
        networksetup_listallnetworkservices: "networksetup -listallnetworkservices",
        networksetup_getinfo: "networksetup -getinfo Wi-Fi",
        networksetup_getdnsservers: "networksetup -getdnsservers Wi-Fi",
        networksetup_version: "networksetup -version",
        networksetup_help: "networksetup -help",
        log_help: "log help",
        log_show: "log show --predicate 'process == \"Safari\"' --last 1h",
        log_show_style: "log show --style compact",
        log_show_info: "log show --info",
        log_show_debug: "log show --debug",
        log_stats: "log stats",
        log_stream: "log stream --level debug",
        log_stream_process: "log stream --process Safari",
    }

    denied! {
        brew_install_denied: "brew install node",
        brew_uninstall_denied: "brew uninstall node",
        brew_services_denied: "brew services list",
        brew_upgrade_denied: "brew upgrade",
        brew_list_unknown_denied: "brew list --unknown",
        brew_info_unknown_denied: "brew info --unknown node",
        brew_search_unknown_denied: "brew search --unknown node",
        brew_deps_unknown_denied: "brew deps --unknown node",
        brew_outdated_unknown_denied: "brew outdated --unknown",
        bare_brew_denied: "brew",
        mise_install_denied: "mise install ruby@3.4",
        mise_exec_denied: "mise exec -- ruby foo.rb",
        mise_use_denied: "mise use ruby@3.4",
        mise_config_set_denied: "mise config set key value",
        mise_config_bare_denied: "mise config",
        mise_ls_unknown_denied: "mise ls --unknown",
        mise_env_unknown_denied: "mise env --unknown",
        asdf_plugin_add_denied: "asdf plugin add ruby",
        asdf_install_denied: "asdf install ruby 3.4",
        asdf_list_unknown_denied: "asdf list ruby --unknown",
        defaults_write_denied: "defaults write com.apple.finder ShowPathbar -bool true",
        defaults_delete_denied: "defaults delete com.apple.finder",
        defaults_read_unknown_denied: "defaults read --unknown",
        pmset_sleep_denied: "pmset sleepnow",
        pmset_set_denied: "pmset -a displaysleep 10",
        bare_pmset_denied: "pmset",
        sysctl_write_denied: "sysctl -w kern.maxproc=2048",
        sysctl_write_long_denied: "sysctl --write kern.maxproc=2048",
        sysctl_assign_denied: "sysctl kern.maxproc=2048",
        sysctl_unknown_denied: "sysctl --unknown kern.maxproc",
        cmake_build_denied: "cmake --build .",
        cmake_generate_denied: "cmake .",
        security_add_denied: "security add-certificates cert.pem",
        security_delete_denied: "security delete-keychain test.keychain",
        security_no_args_denied: "security",
        security_find_cert_unknown_denied: "security find-certificate --unknown",
        security_find_identity_unknown_denied: "security find-identity --unknown",
        security_dump_keychain_decrypt_denied: "security dump-keychain -d",
        security_find_password_g_denied: "security find-generic-password -g",
        security_find_password_w_denied: "security find-internet-password -w pass",
        csrutil_enable_denied: "csrutil enable",
        csrutil_disable_denied: "csrutil disable",
        csrutil_clear_denied: "csrutil clear",
        csrutil_no_args_denied: "csrutil",
        csrutil_status_unknown_denied: "csrutil status --unknown",
        launchctl_load_denied: "launchctl load /Library/LaunchDaemons/foo.plist",
        launchctl_start_denied: "launchctl start com.apple.Finder",
        launchctl_stop_denied: "launchctl stop com.apple.Finder",
        launchctl_no_args_denied: "launchctl",
        launchctl_list_unknown_denied: "launchctl list --unknown",
        diskutil_erase_denied: "diskutil eraseDisk JHFS+ Untitled disk2",
        diskutil_mount_denied: "diskutil mount disk2s1",
        diskutil_unmount_denied: "diskutil unmount disk2s1",
        diskutil_apfs_delete_denied: "diskutil apfs deleteVolume disk1s2",
        diskutil_no_args_denied: "diskutil",
        diskutil_apfs_bare_denied: "diskutil apfs",
        diskutil_list_unknown_denied: "diskutil list --unknown",
        diskutil_info_unknown_denied: "diskutil info --unknown disk0",
        networksetup_setdnsservers_denied: "networksetup -setdnsservers Wi-Fi 8.8.8.8",
        networksetup_setairportpower_denied: "networksetup -setairportpower en0 on",
        networksetup_no_args_denied: "networksetup",
        log_config_denied: "log config --mode level:debug",
        log_erase_denied: "log erase --all",
        log_no_args_denied: "log",
        log_show_unknown_denied: "log show --unknown",
        log_stream_unknown_denied: "log stream --unknown",
    }
}
