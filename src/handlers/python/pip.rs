use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static PIP_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--editable", "--exclude-editable", "--include-editable",
        "--local", "--not-required", "--outdated", "--pre",
        "--uptodate", "--user",
    ]),
    standalone_short: b"eilo",
    valued: WordSet::new(&[
        "--exclude", "--format", "--index-url", "--path",
    ]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static PIP_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--files", "--verbose"]),
    standalone_short: b"fv",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static PIP_FREEZE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--exclude-editable", "--local", "--user",
    ]),
    standalone_short: b"l",
    valued: WordSet::new(&["--exclude", "--path"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static PIP_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static PIP_SUBS: &[SubDef] = &[
    SubDef::Policy { name: "list", policy: &PIP_LIST_POLICY },
    SubDef::Policy { name: "show", policy: &PIP_SHOW_POLICY },
    SubDef::Policy { name: "freeze", policy: &PIP_FREEZE_POLICY },
    SubDef::Policy { name: "check", policy: &PIP_BARE_POLICY },
    SubDef::Nested { name: "config", subs: &[
        SubDef::Policy { name: "get", policy: &PIP_BARE_POLICY },
        SubDef::Policy { name: "list", policy: &PIP_BARE_POLICY },
    ]},
    SubDef::Policy { name: "debug", policy: &PIP_BARE_POLICY },
    SubDef::Policy { name: "help", policy: &PIP_BARE_POLICY },
    SubDef::Policy { name: "index", policy: &PIP_BARE_POLICY },
    SubDef::Policy { name: "inspect", policy: &PIP_BARE_POLICY },
];

pub(crate) static PIP: CommandDef = CommandDef {
    name: "pip",
    subs: PIP_SUBS,
    bare_flags: &[],
    help_eligible: true,
};

pub(crate) static PIP3: CommandDef = CommandDef {
    name: "pip3",
    subs: PIP_SUBS,
    bare_flags: &[],
    help_eligible: true,
};

pub(in crate::handlers::python) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut pip_doc = PIP.to_doc();
    pip_doc.name = "pip / pip3";
    vec![pip_doc]
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
