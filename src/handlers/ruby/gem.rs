use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static GEM_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--help", "--installed", "--local", "--no-details",
        "--no-versions", "--prerelease", "--remote", "--versions",
        "-a", "-d", "-h", "-i", "-l", "-r",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GEM_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--installed", "--prerelease", "-h", "-i"]),
    valued: WordSet::flags(&["--version", "-v"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GEM_SEARCH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--details", "--exact", "--help", "--local",
        "--prerelease", "--remote", "--versions",
        "-a", "-d", "-e", "-h", "-i", "-l", "-r",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GEM_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--help", "--local", "--prerelease", "--remote", "--versions",
        "-a", "-h", "-i", "-l", "-r",
    ]),
    valued: WordSet::flags(&["--version", "-v"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static GEM: CommandDef = CommandDef {
    name: "gem",
    subs: &[
        SubDef::Policy { name: "contents", policy: &GEM_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "dependency", policy: &GEM_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "environment", policy: &GEM_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "help", policy: &GEM_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "info", policy: &GEM_INFO_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "list", policy: &GEM_LIST_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "outdated", policy: &GEM_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "pristine", policy: &GEM_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "search", policy: &GEM_SEARCH_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "sources", policy: &GEM_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "specification", policy: &GEM_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "stale", policy: &GEM_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "which", policy: &GEM_SIMPLE_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://guides.rubygems.org/command-reference/",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        gem_list: "gem list",
        gem_list_local: "gem list --local",
        gem_list_remote: "gem list --remote",
        gem_list_all: "gem list --all",
        gem_info: "gem info rails",
        gem_info_installed: "gem info rails --installed",
        gem_environment: "gem environment",
        gem_which: "gem which bundler",
        gem_pristine: "gem pristine --all",
        gem_search: "gem search rails",
        gem_search_remote: "gem search rails --remote",
        gem_search_exact: "gem search rails --exact",
        gem_specification: "gem specification rails",
        gem_dependency: "gem dependency rails",
        gem_contents: "gem contents rails",
        gem_sources: "gem sources",
        gem_stale: "gem stale",
        gem_outdated: "gem outdated",
        gem_help: "gem help",
        gem_version: "gem --version",
    }
}
