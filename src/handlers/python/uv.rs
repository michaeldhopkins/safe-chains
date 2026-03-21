use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static UV_PIP_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--editable", "--exclude-editable", "--help", "--outdated",
        "--strict",
        "-h",
    ]),
    valued: WordSet::flags(&["--exclude", "--format", "--python"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static UV_PIP_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--files", "--help", "--verbose", "-h", "-v"]),
    valued: WordSet::flags(&["--python"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static UV_PIP_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--verbose", "-h", "-v"]),
    valued: WordSet::flags(&["--python"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static UV_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--verbose", "-h", "-v"]),
    valued: WordSet::flags(&["--python"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static UV: CommandDef = CommandDef {
    name: "uv",
    subs: &[
        SubDef::Nested { name: "pip", subs: &[
            SubDef::Policy { name: "list", policy: &UV_PIP_LIST_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "show", policy: &UV_PIP_SHOW_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "check", policy: &UV_PIP_SIMPLE_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "freeze", policy: &UV_PIP_SIMPLE_POLICY, level: SafetyLevel::Inert },
        ]},
        SubDef::Nested { name: "python", subs: &[
            SubDef::Policy { name: "list", policy: &UV_SIMPLE_POLICY, level: SafetyLevel::Inert },
        ]},
        SubDef::Nested { name: "tool", subs: &[
            SubDef::Policy { name: "list", policy: &UV_SIMPLE_POLICY, level: SafetyLevel::Inert },
        ]},
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://docs.astral.sh/uv/reference/cli/",
    aliases: &[],
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
