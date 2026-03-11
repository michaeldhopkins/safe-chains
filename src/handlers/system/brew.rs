use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static BREW_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--cask", "--formula", "--full-name", "--multiple",
        "--pinned", "--versions",
        "-1", "-l", "-r", "-t",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BREW_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--analytics", "--cask", "--formula", "--installed", "--json",
        "-v",
    ]),
    valued: WordSet::flags(&["--days"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BREW_SEARCH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--cask", "--closed", "--debian", "--desc", "--fedora",
        "--fink", "--formula", "--macports", "--open",
        "--opensuse", "--pull-request", "--repology", "--ubuntu",
    ]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BREW_DEPS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--1", "--annotate", "--cask", "--direct", "--for-each",
        "--formula", "--full-name", "--graph", "--include-build",
        "--include-optional", "--include-test", "--installed", "--missing",
        "--skip-recommended", "--tree", "--union",
        "-n",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BREW_USES_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--cask", "--formula", "--include-build", "--include-optional",
        "--include-test", "--installed", "--missing",
        "--recursive", "--skip-recommended",
    ]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BREW_OUTDATED_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--cask", "--fetch-HEAD", "--formula", "--greedy",
        "--greedy-auto-updates", "--greedy-latest", "--json",
        "-d", "-q", "-v",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BREW_DESC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--cask", "--description", "--eval-all", "--formula",
        "--name", "--search",
        "-d", "-n", "-s",
    ]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BREW_LOG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--cask", "--formula", "--oneline",
        "-1",
    ]),
    valued: WordSet::flags(&["--max-count", "-n"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BREW_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["-q", "-v"]),
    valued: WordSet::flags(&[]),
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
        SubDef::Policy { name: "--repository", policy: &BREW_SIMPLE_POLICY },
        SubDef::Policy { name: "tap-info", policy: &BREW_SIMPLE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://docs.brew.sh/Manpage",
    aliases: &[],
};

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
        brew_tap_info: "brew tap-info homebrew/core",
        brew_tap_info_quiet: "brew tap-info -q homebrew/core",
        brew_repository: "brew --repository",
        brew_repository_tap: "brew --repository homebrew/core",
    }
}
