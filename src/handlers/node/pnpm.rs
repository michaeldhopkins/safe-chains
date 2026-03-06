use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static PNPM_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--dev", "--json", "--long", "--no-optional",
        "--parseable", "--production", "--recursive",
    ]),
    standalone_short: b"Pr",
    valued: WordSet::new(&["--depth", "--filter"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static PNPM_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json", "--recursive"]),
    standalone_short: b"r",
    valued: WordSet::new(&["--filter"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static PNPM: CommandDef = CommandDef {
    name: "pnpm",
    subs: &[
        SubDef::Policy { name: "list", policy: &PNPM_LIST_POLICY },
        SubDef::Policy { name: "ls", policy: &PNPM_LIST_POLICY },
        SubDef::Policy { name: "audit", policy: &PNPM_BARE_POLICY },
        SubDef::Policy { name: "outdated", policy: &PNPM_BARE_POLICY },
        SubDef::Policy { name: "why", policy: &PNPM_BARE_POLICY },
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
        pnpm_list: "pnpm list",
        pnpm_list_json: "pnpm list --json",
        pnpm_list_depth: "pnpm list --depth 0",
        pnpm_why: "pnpm why lodash",
        pnpm_audit: "pnpm audit",
        pnpm_outdated: "pnpm outdated",
        pnpm_version: "pnpm --version",
    }
}
