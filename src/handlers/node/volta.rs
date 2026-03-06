use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static VOLTA_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--current", "--default"]),
    standalone_short: b"cd",
    valued: WordSet::new(&["--format"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static VOLTA: CommandDef = CommandDef {
    name: "volta",
    subs: &[
        SubDef::Policy { name: "list", policy: &VOLTA_BARE_POLICY },
        SubDef::Policy { name: "which", policy: &VOLTA_BARE_POLICY },
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
        volta_list: "volta list",
        volta_list_current: "volta list --current",
        volta_which: "volta which node",
        volta_version: "volta --version",
    }
}
