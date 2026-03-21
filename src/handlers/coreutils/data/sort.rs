use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SORT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--check", "--debug", "--dictionary-order",
        "--general-numeric-sort", "--help", "--human-numeric-sort",
        "--ignore-case", "--ignore-leading-blanks",
        "--ignore-nonprinting", "--merge", "--month-sort",
        "--numeric-sort", "--random-sort", "--reverse",
        "--stable", "--unique", "--version", "--version-sort",
        "--zero-terminated",
        "-C", "-M", "-R", "-V", "-b", "-c", "-d",
        "-f", "-g", "-h", "-i", "-m", "-n", "-r",
        "-s", "-u", "-z",
    ]),
    valued: WordSet::flags(&[
        "--batch-size", "--buffer-size", "--field-separator",
        "--files0-from", "--key", "--parallel",
        "--random-source", "--sort", "--temporary-directory",
        "-S", "-T", "-k", "-t",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "sort", policy: &SORT_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#sort-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        sort_basic: "sort file.txt",
        sort_reverse: "sort -r file.txt",
        sort_n_u: "sort -n -u file.txt",
        sort_field: "sort -t: -k2 /etc/passwd",
    }

    denied! {
        sort_output_denied: "sort -o output.txt file.txt",
        sort_output_long_denied: "sort --output=result.txt file.txt",
        sort_output_long_space_denied: "sort --output result.txt file.txt",
        sort_rno_combined_denied: "sort -rno sorted.txt file.txt",
        sort_compress_program_denied: "sort --compress-program sh file.txt",
        sort_compress_program_eq_denied: "sort --compress-program=gzip file.txt",
        sort_output_trailing_help_denied: "sort -o output.txt file --help",
        sort_output_trailing_version_denied: "sort -o output.txt file --version",
    }
}
