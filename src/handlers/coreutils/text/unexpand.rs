use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static UNEXPAND_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--first-only",
        "-a",
    ]),
    standalone_short: b"a",
    valued: WordSet::new(&[
        "--tabs",
        "-t",
    ]),
    valued_short: b"t",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "unexpand", policy: &UNEXPAND_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        unexpand_file: "unexpand file.txt",
        unexpand_all: "unexpand -a file.txt",
        unexpand_tabs: "unexpand --tabs 8 file.txt",
        unexpand_bare: "unexpand",
    }
}
