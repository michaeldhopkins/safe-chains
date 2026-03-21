use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static BRANCHDIFF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--diff", "--help", "--no-auto-fetch", "--print", "--version",
        "-V", "-d", "-h", "-p",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(1),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "branchdiff", policy: &BRANCHDIFF_POLICY, level: SafetyLevel::Inert, url: "https://github.com/michaeldhopkins/branchdiff", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        branchdiff_bare: "branchdiff",
        branchdiff_help: "branchdiff --help",
        branchdiff_help_short: "branchdiff -h",
        branchdiff_version: "branchdiff --version",
        branchdiff_version_short: "branchdiff -V",
        branchdiff_print: "branchdiff --print",
        branchdiff_print_short: "branchdiff -p",
        branchdiff_diff: "branchdiff --diff",
        branchdiff_diff_short: "branchdiff -d",
        branchdiff_no_auto_fetch: "branchdiff --no-auto-fetch",
        branchdiff_print_path: "branchdiff --print /path/to/repo",
        branchdiff_diff_path: "branchdiff -d /path/to/repo",
        branchdiff_path: "branchdiff /path/to/repo",
    }

    denied! {
        branchdiff_benchmark: "branchdiff --benchmark 100",
        branchdiff_unknown: "branchdiff --unknown",
        branchdiff_two_paths: "branchdiff /a /b",
    }
}
