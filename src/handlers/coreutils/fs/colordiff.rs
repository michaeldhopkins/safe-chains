use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static COLORDIFF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--brief", "--ed", "--expand-tabs", "--help", "--initial-tab",
        "--left-column", "--minimal", "--normal",
        "--paginate", "--rcs", "--report-identical-files",
        "--side-by-side", "--speed-large-files",
        "--strip-trailing-cr", "--suppress-blank-empty",
        "--suppress-common-lines", "--text", "--version",
        "-B", "-E", "-N", "-P", "-T", "-V", "-Z",
        "-a", "-b", "-c", "-d", "-e", "-f", "-h", "-i", "-l", "-n",
        "-p", "-q", "-r", "-s", "-t", "-u", "-v", "-w", "-y",
    ]),
    valued: WordSet::flags(&[
        "--changed-group-format", "--color", "--context",
        "--from-file", "--horizon-lines", "--ifdef",
        "--ignore-matching-lines", "--label", "--line-format",
        "--new-group-format", "--new-line-format",
        "--old-group-format", "--old-line-format",
        "--show-function-line", "--starting-file",
        "--tabsize", "--to-file", "--unchanged-group-format",
        "--unchanged-line-format", "--unified", "--width",
        "-C", "-D", "-F", "-I", "-L", "-S", "-U", "-W",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "colordiff", policy: &COLORDIFF_POLICY, level: SafetyLevel::Inert, url: "https://www.colordiff.org/", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        colordiff_files: "colordiff file1 file2",
        colordiff_unified: "colordiff -u file1 file2",
    }
}
