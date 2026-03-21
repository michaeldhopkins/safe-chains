use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static PIP_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--editable", "--exclude-editable", "--help", "--include-editable",
        "--local", "--not-required", "--outdated", "--pre",
        "--uptodate", "--user",
        "-e", "-h", "-i", "-l", "-o",
    ]),
    valued: WordSet::flags(&[
        "--exclude", "--format", "--index-url", "--path",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static PIP_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--files", "--help", "--verbose", "-f", "-h", "-v"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static PIP_FREEZE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--exclude-editable", "--help", "--local", "--user",
        "-h", "-l",
    ]),
    valued: WordSet::flags(&["--exclude", "--path"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static PIP_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static PIP_SUBS: &[SubDef] = &[
    SubDef::Policy { name: "list", policy: &PIP_LIST_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "show", policy: &PIP_SHOW_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "freeze", policy: &PIP_FREEZE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "check", policy: &PIP_BARE_POLICY, level: SafetyLevel::Inert },
    SubDef::Nested { name: "config", subs: &[
        SubDef::Policy { name: "get", policy: &PIP_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "list", policy: &PIP_BARE_POLICY, level: SafetyLevel::Inert },
    ]},
    SubDef::Policy { name: "debug", policy: &PIP_BARE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "help", policy: &PIP_BARE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "index", policy: &PIP_BARE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "inspect", policy: &PIP_BARE_POLICY, level: SafetyLevel::Inert },
];

pub(crate) static PIP: CommandDef = CommandDef {
    name: "pip",
    subs: PIP_SUBS,
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://pip.pypa.io/en/stable/cli/",
    aliases: &["pip3"],
};

pub(in crate::handlers::python) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![PIP.to_doc()]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        pip_list: "pip list",
        pip_list_outdated: "pip list --outdated",
        pip_list_format: "pip list --format json",
        pip_show: "pip show requests",
        pip_show_files: "pip show requests --files",
        pip_freeze: "pip freeze",
        pip_freeze_all: "pip freeze --all",
        pip_check: "pip check",
        pip_index: "pip index versions requests",
        pip_debug: "pip debug",
        pip_inspect: "pip inspect",
        pip_help: "pip help",
        pip_config_list: "pip config list",
        pip_config_get: "pip config get global.index-url",
        pip3_list: "pip3 list",
        pip3_show: "pip3 show flask",
        pip3_freeze: "pip3 freeze",
        pip_version: "pip --version",
        pip3_version: "pip3 --version",
    }

    denied! {
        pip_config_set_denied: "pip config set global.index-url https://example.com",
    }
}
