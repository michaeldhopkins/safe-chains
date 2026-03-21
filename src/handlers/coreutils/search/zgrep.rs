use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static ZGREP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--count", "--extended-regexp", "--files-with-matches",
        "--files-without-match", "--fixed-strings", "--help", "--ignore-case",
        "--invert-match", "--line-number", "--no-filename",
        "--only-matching", "--quiet", "--silent", "--version", "--with-filename",
        "--word-regexp",
        "-E", "-F", "-G", "-H", "-L", "-V", "-Z",
        "-c", "-h", "-i", "-l", "-n", "-o", "-q", "-s", "-v", "-w", "-x",
    ]),
    valued: WordSet::flags(&[
        "--after-context", "--before-context", "--context",
        "--file", "--max-count", "--regexp",
        "-A", "-B", "-C", "-e", "-f", "-m",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "zgrep", policy: &ZGREP_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/zgrep.1.html", aliases: &["zegrep", "zfgrep"] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        zgrep_basic: "zgrep pattern file.gz",
        zgrep_ignore_case: "zgrep -i pattern file.gz",
        zgrep_count: "zgrep -c pattern file.gz",
        zgrep_line_number: "zgrep -n pattern file.gz",
        zgrep_context: "zgrep -C 3 pattern file.gz",
        zegrep_basic: "zegrep 'foo|bar' file.gz",
        zfgrep_basic: "zfgrep exact file.gz",
    }
}
