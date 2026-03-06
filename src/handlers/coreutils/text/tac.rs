use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static TAC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--before", "--regex",
        "-b", "-r",
    ]),
    standalone_short: b"br",
    valued: WordSet::new(&[
        "--separator",
        "-s",
    ]),
    valued_short: b"s",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "tac", policy: &TAC_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        tac_file: "tac file.txt",
        tac_bare: "tac",
        tac_separator: "tac -s '---' file",
        tac_before: "tac -b file",
    }
}
