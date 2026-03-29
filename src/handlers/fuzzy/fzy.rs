use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};
use crate::verdict::SafetyLevel;

static FZY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--show-info", "--show-scores", "--version",
        "-h", "-i", "-v",
    ]),
    valued: WordSet::flags(&[
        "--lines", "--prompt", "--query", "--show-matches", "--tty",
        "-e", "-l", "-p", "-q", "-t",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(super) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef {
        name: "fzy",
        policy: &FZY_POLICY,
        level: SafetyLevel::Inert,
        url: "https://github.com/jhawthorn/fzy",
        aliases: &[],
    },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        fzy_bare: "fzy",
        fzy_help: "fzy --help",
        fzy_version: "fzy --version",
        fzy_lines: "fzy -l 20",
        fzy_prompt: "fzy -p '> '",
        fzy_query: "fzy -q pattern",
        fzy_show_scores: "fzy --show-scores",
        fzy_show_matches: "fzy -e pattern",
    }
}
