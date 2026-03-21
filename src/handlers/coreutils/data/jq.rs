use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static JQ_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--ascii-output", "--color-output", "--compact-output", "--exit-status", "--help",
        "--join-output",
        "--monochrome-output", "--null-input", "--raw-input", "--raw-output", "--raw-output0",
        "--seq", "--slurp", "--sort-keys", "--tab", "--version", "-C",
        "-M", "-R", "-S", "-V", "-c", "-e",
        "-g", "-h", "-j", "-n", "-r", "-s",
    ]),
    valued: WordSet::flags(&[
        "--arg", "--argjson", "--args", "--from-file",
        "--indent", "--jsonargs", "--rawfile",
        "--slurpfile", "-f",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "jq", policy: &JQ_POLICY, level: SafetyLevel::Inert, url: "https://jqlang.github.io/jq/manual/", aliases: &[] },
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
