use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};
use crate::verdict::SafetyLevel;

static PECO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--null", "--print-query", "--select-1", "--version",
        "-h", "-v",
    ]),
    valued: WordSet::flags(&[
        "--buffer-size", "--initial-filter", "--initial-index",
        "--layout", "--on-cancel", "--prompt", "--query",
        "--selection-prefix",
        "-b",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(super) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef {
        name: "peco",
        policy: &PECO_POLICY,
        level: SafetyLevel::Inert,
        url: "https://github.com/peco/peco",
        aliases: &[],
    },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        peco_bare: "peco",
        peco_help: "peco --help",
        peco_version: "peco --version",
        peco_query: "peco --query pattern",
        peco_layout: "peco --layout=bottom-up",
        peco_prompt: "peco --prompt '> '",
        peco_null: "peco --null",
        peco_select_1: "peco --select-1",
        peco_buffer_size: "peco --buffer-size 100",
    }

    denied! {
        peco_exec: "peco --exec 'cat {}'",
        peco_rcfile: "peco --rcfile /tmp/evil.json",
    }
}
