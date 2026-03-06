use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static JQ_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--ascii-output", "--color-output", "--compact-output", "--exit-status", "--join-output",
        "--monochrome-output", "--null-input", "--raw-input", "--raw-output", "--raw-output0",
        "--seq", "--slurp", "--sort-keys", "--tab", "-C",
        "-M", "-R", "-S", "-c", "-e",
        "-j", "-n", "-r", "-s",
    ]),
    standalone_short: b"CMRScegjnrs",
    valued: WordSet::new(&[
        "--arg", "--argjson", "--args", "--from-file",
        "--indent", "--jsonargs", "--rawfile",
        "--slurpfile", "-f",
    ]),
    valued_short: b"f",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "jq", policy: &JQ_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        jq_filter: "jq '.name' file.json",
        jq_compact: "jq -c . file.json",
        jq_raw: "jq -r '.url' file.json",
        jq_slurp: "jq -s '.[0]' file.json",
    }
}
