use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static POETRY_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--latest", "--no-dev", "--outdated",
        "--top-level", "--tree",
    ]),
    standalone_short: b"loT",
    valued: WordSet::new(&["--why"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static POETRY_CHECK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--lock"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static POETRY_ENV_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--full-path"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static POETRY: CommandDef = CommandDef {
    name: "poetry",
    subs: &[
        SubDef::Policy { name: "show", policy: &POETRY_SHOW_POLICY },
        SubDef::Policy { name: "check", policy: &POETRY_CHECK_POLICY },
        SubDef::Nested { name: "env", subs: &[
            SubDef::Policy { name: "info", policy: &POETRY_ENV_POLICY },
            SubDef::Policy { name: "list", policy: &POETRY_ENV_POLICY },
        ]},
    ],
    bare_flags: &[],
    help_eligible: true,
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        poetry_show: "poetry show",
        poetry_show_tree: "poetry show --tree",
        poetry_show_outdated: "poetry show --outdated",
        poetry_show_latest: "poetry show --latest",
        poetry_check: "poetry check",
        poetry_check_lock: "poetry check --lock",
        poetry_version: "poetry --version",
        poetry_env_info: "poetry env info",
        poetry_env_info_full: "poetry env info --full-path",
        poetry_env_list: "poetry env list",
    }
}
