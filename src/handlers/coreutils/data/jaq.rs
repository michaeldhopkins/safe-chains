use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static JAQ_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--compact-output", "--exit-status", "--help",
        "--join-output", "--null-input", "--raw-input",
        "--raw-output", "--slurp", "--sort-keys", "--tab",
        "--version",
        "-C", "-M", "-R", "-S", "-V", "-c", "-e", "-h", "-j", "-n", "-r", "-s",
    ]),
    valued: WordSet::flags(&[
        "--arg", "--argjson", "--from-file", "--indent",
        "--rawfile", "--slurpfile", "-f",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "jaq", policy: &JAQ_POLICY, level: SafetyLevel::Inert, url: "https://github.com/01mf02/jaq", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        jaq_filter: "jaq '.name' file.json",
        jaq_compact: "jaq -c . file.json",
        jaq_raw: "jaq -r '.url' file.json",
        jaq_slurp: "jaq -s '.[0]' file.json",
    }
}
