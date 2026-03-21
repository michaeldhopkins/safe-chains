use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static TOKEI_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--compact", "--files", "--help", "--hidden", "--no-ignore",
        "--no-ignore-dot", "--no-ignore-parent",
        "--no-ignore-vcs", "--verbose", "--version",
        "-C", "-V", "-f", "-h",
    ]),
    valued: WordSet::flags(&[
        "--columns", "--exclude", "--input",
        "--languages", "--num-format", "--output",
        "--sort", "--type",
        "-c", "-e", "-i", "-l", "-o", "-s", "-t",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "tokei", policy: &TOKEI_POLICY, level: SafetyLevel::Inert, url: "https://github.com/XAMPPRocky/tokei#readme", aliases: &[] },
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
