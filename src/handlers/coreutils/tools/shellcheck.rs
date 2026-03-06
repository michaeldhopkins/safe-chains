use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SHELLCHECK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--color", "--external-sources", "--list-optional",
        "--norc", "--severity", "--wiki-link-count",
        "-C", "-a", "-x",
    ]),
    standalone_short: b"Cax",
    valued: WordSet::new(&[
        "--enable", "--exclude", "--format", "--include",
        "--rcfile", "--severity", "--shell", "--source-path",
        "--wiki-link-count",
        "-P", "-S", "-W", "-e", "-f", "-i", "-o", "-s",
    ]),
    valued_short: b"PSWefios",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "shellcheck", policy: &SHELLCHECK_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        shellcheck_file: "shellcheck script.sh",
        shellcheck_format: "shellcheck -f json script.sh",
    }
}
