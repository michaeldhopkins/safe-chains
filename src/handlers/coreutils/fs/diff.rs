use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DIFF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--brief", "--ed", "--expand-tabs", "--ignore-all-space",
        "--ignore-blank-lines", "--ignore-case", "--ignore-space-change",
        "--ignore-tab-expansion", "--left-column", "--minimal",
        "--new-file", "--no-dereference", "--no-ignore-file-name-case",
        "--normal", "--paginate", "--rcs", "--recursive",
        "--report-identical-files", "--show-c-function", "--side-by-side",
        "--speed-large-files", "--strip-trailing-cr",
        "--suppress-blank-empty", "--suppress-common-lines", "--text",
        "--unidirectional-new-file",
        "-B", "-E", "-N", "-P", "-T",
        "-a", "-b", "-c", "-d", "-e", "-f", "-i", "-l", "-n", "-p",
        "-q", "-r", "-s", "-t", "-u", "-w", "-y",
    ]),
    standalone_short: b"BENPTabcdefilnpqrstuwy",
    valued: WordSet::new(&[
        "--changed-group-format", "--color", "--context", "--exclude",
        "--exclude-from", "--from-file", "--ifdef", "--ignore-matching-lines",
        "--label", "--line-format", "--new-group-format", "--new-line-format",
        "--old-group-format", "--old-line-format", "--show-function-line",
        "--starting-file", "--tabsize", "--to-file", "--unchanged-group-format",
        "--unchanged-line-format", "--unified", "--width",
        "-C", "-D", "-F", "-I", "-L", "-S", "-U", "-W", "-X", "-x",
    ]),
    valued_short: b"CDFILSUWXx",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "diff", policy: &DIFF_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        diff_files: "diff file1.txt file2.txt",
        diff_unified: "diff -u file1 file2",
        diff_context: "diff -C 3 file1 file2",
        diff_recursive: "diff -r dir1 dir2",
        diff_brief: "diff --brief dir1 dir2",
        diff_side: "diff -y file1 file2",
        diff_color: "diff --color file1 file2",
    }
}
