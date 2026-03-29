use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};
use crate::verdict::SafetyLevel;

static SELECTA_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--version",
        "-h", "-v",
    ]),
    valued: WordSet::flags(&[
        "--search",
        "-s",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(super) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef {
        name: "selecta",
        policy: &SELECTA_POLICY,
        level: SafetyLevel::Inert,
        url: "https://github.com/garybernhardt/selecta",
        aliases: &[],
    },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        selecta_bare: "selecta",
        selecta_help: "selecta --help",
        selecta_search: "selecta --search pattern",
    }
}
