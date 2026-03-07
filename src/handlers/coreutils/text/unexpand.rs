use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static UNEXPAND_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--first-only",
        "-a",
    ]),
    valued: WordSet::flags(&[
        "--tabs",
        "-t",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "unexpand", policy: &UNEXPAND_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#unexpand-invocation" },
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
