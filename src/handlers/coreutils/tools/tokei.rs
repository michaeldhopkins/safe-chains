use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static TOKEI_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--compact", "--files", "--hidden", "--no-ignore",
        "--no-ignore-dot", "--no-ignore-parent",
        "--no-ignore-vcs", "--verbose",
        "-C", "-V", "-f",
    ]),
    standalone_short: b"CVf",
    valued: WordSet::flags(&[
        "--columns", "--exclude", "--input",
        "--languages", "--num-format", "--output",
        "--sort", "--type",
        "-c", "-e", "-i", "-l", "-o", "-s", "-t",
    ]),
    valued_short: b"ceilost",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "tokei", policy: &TOKEI_POLICY, help_eligible: false, url: "https://github.com/XAMPPRocky/tokei#readme" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        tokei_bare: "tokei",
        tokei_sort: "tokei -s lines",
    }
}
