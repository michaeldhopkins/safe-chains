use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};
use crate::verdict::SafetyLevel;

static PICK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--version",
        "-K", "-S", "-X", "-d", "-h", "-o", "-v", "-x",
    ]),
    valued: WordSet::flags(&[
        "-q",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(super) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef {
        name: "pick",
        policy: &PICK_POLICY,
        level: SafetyLevel::Inert,
        url: "https://github.com/mptre/pick",
        aliases: &[],
    },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        pick_bare: "pick",
        pick_help: "pick --help",
        pick_version: "pick --version",
        pick_descriptions: "pick -d",
        pick_query: "pick -q pattern",
        pick_no_sort: "pick -S",
    }
}
