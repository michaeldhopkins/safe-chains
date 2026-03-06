use crate::command::{CheckFn, CommandDef, SubDef};
use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
};

static BREW_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"qv",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static BREW: CommandDef = CommandDef {
    name: "brew",
    subs: &[
        SubDef::Policy { name: "list", policy: &BREW_LIST_POLICY },
        SubDef::Policy { name: "ls", policy: &BREW_LIST_POLICY },
        SubDef::Policy { name: "info", policy: &BREW_INFO_POLICY },
        SubDef::Policy { name: "abv", policy: &BREW_INFO_POLICY },
        SubDef::Policy { name: "search", policy: &BREW_SEARCH_POLICY },
        SubDef::Policy { name: "deps", policy: &BREW_DEPS_POLICY },
        SubDef::Policy { name: "uses", policy: &BREW_USES_POLICY },
        SubDef::Policy { name: "outdated", policy: &BREW_OUTDATED_POLICY },
        SubDef::Policy { name: "desc", policy: &BREW_DESC_POLICY },
        SubDef::Policy { name: "log", policy: &BREW_LOG_POLICY },
        SubDef::Policy { name: "cat", policy: &BREW_SIMPLE_POLICY },
        SubDef::Policy { name: "casks", policy: &BREW_SIMPLE_POLICY },
        SubDef::Policy { name: "config", policy: &BREW_SIMPLE_POLICY },
        SubDef::Policy { name: "doctor", policy: &BREW_SIMPLE_POLICY },
        SubDef::Policy { name: "formulae", policy: &BREW_SIMPLE_POLICY },
        SubDef::Policy { name: "home", policy: &BREW_SIMPLE_POLICY },
        SubDef::Policy { name: "leaves", policy: &BREW_SIMPLE_POLICY },
        SubDef::Policy { name: "shellenv", policy: &BREW_SIMPLE_POLICY },
        SubDef::Policy { name: "tap", policy: &BREW_SIMPLE_POLICY },
        SubDef::Policy { name: "--prefix", policy: &BREW_SIMPLE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

static MISE_RESHIM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--force"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static MISE_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"qv",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
};

static MISE_ENV_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json"]),
    standalone_short: b"J",
    valued: WordSet::new(&["--shell"]),
    valued_short: b"s",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn check_mise_exec(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    let sep = tokens[1..].iter().position(|t| *t == "--");
    if let Some(pos) = sep {
        let inner_start = 1 + pos + 1;
        if inner_start >= tokens.len() {
            return false;
        }
        let inner = Token::join(&tokens[inner_start..]);
        return is_safe(&inner);
    }
    false
}

pub(crate) static MISE: CommandDef = CommandDef {
    name: "mise",
    subs: &[
        SubDef::Policy { name: "list", policy: &MISE_LIST_POLICY },
        SubDef::Policy { name: "ls", policy: &MISE_LIST_POLICY },
        SubDef::Policy { name: "current", policy: &MISE_SIMPLE_POLICY },
        SubDef::Policy { name: "which", policy: &MISE_SIMPLE_POLICY },
        SubDef::Policy { name: "doctor", policy: &MISE_SIMPLE_POLICY },
        SubDef::Policy { name: "reshim", policy: &MISE_RESHIM_POLICY },
        SubDef::Policy { name: "env", policy: &MISE_ENV_POLICY },
        SubDef::Nested { name: "config", subs: &[
            SubDef::Policy { name: "list", policy: &MISE_SIMPLE_POLICY },
            SubDef::Policy { name: "ls", policy: &MISE_SIMPLE_POLICY },
        ]},
        SubDef::Nested { name: "settings", subs: &[
            SubDef::Policy { name: "get", policy: &MISE_SIMPLE_POLICY },
        ]},
        SubDef::Custom { name: "exec", check: check_mise_exec as CheckFn, doc: "exec delegates after --.", test_suffix: None },
    ],
    bare_flags: &[],
    help_eligible: true,
};

static ASDF_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static ASDF: CommandDef = CommandDef {
    name: "asdf",
    subs: &[
        SubDef::Policy { name: "current", policy: &ASDF_SIMPLE_POLICY },
        SubDef::Policy { name: "help", policy: &ASDF_SIMPLE_POLICY },
        SubDef::Policy { name: "info", policy: &ASDF_SIMPLE_POLICY },
        SubDef::Policy { name: "list", policy: &ASDF_SIMPLE_POLICY },
        SubDef::Policy { name: "version", policy: &ASDF_SIMPLE_POLICY },
        SubDef::Policy { name: "which", policy: &ASDF_SIMPLE_POLICY },
        SubDef::Nested { name: "plugin", subs: &[
            SubDef::Policy { name: "list", policy: &ASDF_SIMPLE_POLICY },
        ]},
        SubDef::Policy { name: "plugin-list", policy: &ASDF_SIMPLE_POLICY },
        SubDef::Policy { name: "plugin-list-all", policy: &ASDF_SIMPLE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

static DEFAULTS_READ_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-g", "-globalDomain"]),
    standalone_short: b"",
    valued: WordSet::new(&["-app"]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DEFAULTS_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static DEFAULTS: CommandDef = CommandDef {
    name: "defaults",
    subs: &[
        SubDef::Policy { name: "read", policy: &DEFAULTS_READ_POLICY },
        SubDef::Policy { name: "read-type", policy: &DEFAULTS_READ_POLICY },
        SubDef::Policy { name: "export", policy: &DEFAULTS_READ_POLICY },
        SubDef::Policy { name: "find", policy: &DEFAULTS_READ_POLICY },
        SubDef::Policy { name: "domains", policy: &DEFAULTS_SIMPLE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

static PMSET_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
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

pub(crate) static CMAKE: CommandDef = CommandDef {
    name: "cmake",
    subs: &[],
    bare_flags: &["--system-information", "--version"],
    help_eligible: true,
};

static SECURITY_FIND_CERT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-Z", "-a", "-p"]),
    standalone_short: b"Zap",
    valued: WordSet::new(&["-c", "-e"]),
    valued_short: b"ce",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SECURITY_FIND_IDENTITY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-v"]),
    standalone_short: b"v",
    valued: WordSet::new(&["-p", "-s"]),
    valued_short: b"ps",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
};

static SECURITY_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-d"]),
    standalone_short: b"d",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SECURITY_DUMP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SECURITY_VERIFY_CERT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-L", "-l", "-q"]),
    standalone_short: b"Llq",
    valued: WordSet::new(&["-c", "-k", "-n", "-p", "-r"]),
    valued_short: b"cknpr",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SECURITY_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static SECURITY: CommandDef = CommandDef {
    name: "security",
    subs: &[
        SubDef::Policy { name: "find-certificate", policy: &SECURITY_FIND_CERT_POLICY },
        SubDef::Policy { name: "find-identity", policy: &SECURITY_FIND_IDENTITY_POLICY },
        SubDef::Policy { name: "find-generic-password", policy: &SECURITY_FIND_PASSWORD_POLICY },
        SubDef::Policy { name: "find-internet-password", policy: &SECURITY_FIND_PASSWORD_POLICY },
        SubDef::Policy { name: "list-keychains", policy: &SECURITY_LIST_POLICY },
        SubDef::Policy { name: "dump-keychain", policy: &SECURITY_DUMP_POLICY },
        SubDef::Policy { name: "dump-trust-settings", policy: &SECURITY_DUMP_POLICY },
        SubDef::Policy { name: "verify-cert", policy: &SECURITY_VERIFY_CERT_POLICY },
        SubDef::Policy { name: "cms", policy: &SECURITY_SIMPLE_POLICY },
        SubDef::Policy { name: "show-keychain-info", policy: &SECURITY_SIMPLE_POLICY },
        SubDef::Policy { name: "smartcard", policy: &SECURITY_SIMPLE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

static CSRUTIL_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static CSRUTIL: CommandDef = CommandDef {
    name: "csrutil",
    subs: &[
        SubDef::Policy { name: "authenticated-root", policy: &CSRUTIL_SIMPLE_POLICY },
        SubDef::Policy { name: "report", policy: &CSRUTIL_SIMPLE_POLICY },
        SubDef::Policy { name: "status", policy: &CSRUTIL_SIMPLE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

static DISKUTIL_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DISKUTIL_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-plist"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DISKUTIL_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-all", "-plist"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static DISKUTIL: CommandDef = CommandDef {
    name: "diskutil",
    subs: &[
        SubDef::Policy { name: "list", policy: &DISKUTIL_LIST_POLICY },
        SubDef::Policy { name: "listFilesystems", policy: &DISKUTIL_LIST_POLICY },
        SubDef::Policy { name: "info", policy: &DISKUTIL_INFO_POLICY },
        SubDef::Policy { name: "activity", policy: &DISKUTIL_SIMPLE_POLICY },
        SubDef::Nested { name: "apfs", subs: &[
            SubDef::Policy { name: "list", policy: &DISKUTIL_SIMPLE_POLICY },
            SubDef::Policy { name: "listCryptoUsers", policy: &DISKUTIL_SIMPLE_POLICY },
            SubDef::Policy { name: "listSnapshots", policy: &DISKUTIL_SIMPLE_POLICY },
            SubDef::Policy { name: "listVolumeGroups", policy: &DISKUTIL_SIMPLE_POLICY },
        ]},
    ],
    bare_flags: &[],
    help_eligible: true,
};

static LAUNCHCTL_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static LAUNCHCTL: CommandDef = CommandDef {
    name: "launchctl",
    subs: &[
        SubDef::Policy { name: "blame", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "dumpstate", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "error", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "examine", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "help", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "hostinfo", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "list", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "print", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "print-cache", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "print-disabled", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "resolveport", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "version", policy: &LAUNCHCTL_SIMPLE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
};

static LOG_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static LOG: CommandDef = CommandDef {
    name: "log",
    subs: &[
        SubDef::Policy { name: "show", policy: &LOG_SHOW_POLICY },
        SubDef::Policy { name: "stream", policy: &LOG_STREAM_POLICY },
        SubDef::Policy { name: "help", policy: &LOG_SIMPLE_POLICY },
        SubDef::Policy { name: "stats", policy: &LOG_SIMPLE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    BREW.dispatch(cmd, tokens, is_safe)
        .or_else(|| MISE.dispatch(cmd, tokens, is_safe))
        .or_else(|| ASDF.dispatch(cmd, tokens, is_safe))
        .or_else(|| DEFAULTS.dispatch(cmd, tokens, is_safe))
        .or_else(|| SECURITY.dispatch(cmd, tokens, is_safe))
        .or_else(|| CSRUTIL.dispatch(cmd, tokens, is_safe))
        .or_else(|| DISKUTIL.dispatch(cmd, tokens, is_safe))
        .or_else(|| LAUNCHCTL.dispatch(cmd, tokens, is_safe))
        .or_else(|| LOG.dispatch(cmd, tokens, is_safe))
        .or_else(|| CMAKE.dispatch(cmd, tokens, is_safe))
        .or_else(|| match cmd {
            "pmset" => Some(is_safe_pmset(tokens)),
            "sysctl" => Some(is_safe_sysctl(tokens)),
            "networksetup" => Some(is_safe_networksetup(tokens)),
            _ => None,
        })
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        BREW.to_doc(),
        MISE.to_doc(),
        ASDF.to_doc(),
        DEFAULTS.to_doc(),
        CommandDoc::handler("pmset",
            "Allowed: -g (get/display settings only)."),
        CommandDoc::handler("sysctl",
            "Read-only usage."),
        CMAKE.to_doc(),
        SECURITY.to_doc(),
        CSRUTIL.to_doc(),
        DISKUTIL.to_doc(),
        LAUNCHCTL.to_doc(),
        CommandDoc::handler("networksetup",
            "Allowed: subcommands starting with -list, -get, -show, -print, \
             plus -version and -help."),
        LOG.to_doc(),
    ]
}

#[cfg(test)]
pub(super) const REGISTRY: &[super::CommandEntry] = &[
    super::CommandEntry::Positional { cmd: "pmset" },
    super::CommandEntry::Custom { cmd: "sysctl", valid_prefix: Some("sysctl kern.maxproc") },
    super::CommandEntry::Positional { cmd: "networksetup" },
];

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
        mise_reshim: "mise reshim",
        mise_reshim_force: "mise reshim --force",
        mise_exec_git_status: "mise exec -- git status",
        mise_exec_node_version: "mise exec node@20 -- node --version",
        mise_exec_bundle_rspec: "mise exec -- bundle exec rspec spec/foo_spec.rb --no-color",
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
        mise_exec_rm_denied: "mise exec -- rm -rf /",
        mise_exec_no_inner_denied: "mise exec --",
        mise_exec_no_separator_denied: "mise exec ruby foo.rb",
        mise_exec_bare_denied: "mise exec",
        mise_exec_ruby_denied: "mise exec -- ruby foo.rb",
        mise_config_bare_denied: "mise config",
        pmset_sleep_denied: "pmset sleepnow",
        pmset_set_denied: "pmset -a displaysleep 10",
        bare_pmset_denied: "pmset",
        sysctl_write_denied: "sysctl -w kern.maxproc=2048",
        sysctl_write_long_denied: "sysctl --write kern.maxproc=2048",
        sysctl_assign_denied: "sysctl kern.maxproc=2048",
        cmake_build_denied: "cmake --build .",
        cmake_generate_denied: "cmake .",
        security_dump_keychain_decrypt_denied: "security dump-keychain -d",
        security_find_password_g_denied: "security find-generic-password -g",
        security_find_password_w_denied: "security find-internet-password -w pass",
        diskutil_apfs_bare_denied: "diskutil apfs",
        networksetup_setdnsservers_denied: "networksetup -setdnsservers Wi-Fi 8.8.8.8",
        networksetup_setairportpower_denied: "networksetup -setairportpower en0 on",
        networksetup_no_args_denied: "networksetup",
    }
}
