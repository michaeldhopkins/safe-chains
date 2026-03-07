use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static UV_PIP_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--editable", "--exclude-editable", "--outdated",
        "--strict",
    ]),
    standalone_short: b"",
    valued: WordSet::flags(&["--exclude", "--format", "--python"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static UV_PIP_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--files", "--verbose"]),
    standalone_short: b"v",
    valued: WordSet::flags(&["--python"]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static UV_PIP_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--verbose"]),
    standalone_short: b"v",
    valued: WordSet::flags(&["--python"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static UV_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--verbose"]),
    standalone_short: b"v",
    valued: WordSet::flags(&["--python"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static UV: CommandDef = CommandDef {
    name: "uv",
    subs: &[
        SubDef::Nested { name: "pip", subs: &[
            SubDef::Policy { name: "list", policy: &UV_PIP_LIST_POLICY },
            SubDef::Policy { name: "show", policy: &UV_PIP_SHOW_POLICY },
            SubDef::Policy { name: "check", policy: &UV_PIP_SIMPLE_POLICY },
            SubDef::Policy { name: "freeze", policy: &UV_PIP_SIMPLE_POLICY },
        ]},
        SubDef::Nested { name: "python", subs: &[
            SubDef::Policy { name: "list", policy: &UV_SIMPLE_POLICY },
        ]},
        SubDef::Nested { name: "tool", subs: &[
            SubDef::Policy { name: "list", policy: &UV_SIMPLE_POLICY },
        ]},
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://docs.astral.sh/uv/reference/cli/",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        uv_version: "uv --version",
        uv_pip_list: "uv pip list",
        uv_pip_list_outdated: "uv pip list --outdated",
        uv_pip_show: "uv pip show requests",
        uv_pip_show_files: "uv pip show requests --files",
        uv_pip_freeze: "uv pip freeze",
        uv_pip_check: "uv pip check",
        uv_tool_list: "uv tool list",
        uv_python_list: "uv python list",
    }
}
