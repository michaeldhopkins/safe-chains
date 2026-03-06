use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static GREP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--basic-regexp", "--binary", "--byte-offset", "--color", "--colour",
        "--count", "--dereference-recursive", "--extended-regexp",
        "--files-with-matches", "--files-without-match", "--fixed-strings",
        "--ignore-case", "--initial-tab", "--invert-match", "--line-buffered",
        "--line-number", "--line-regexp", "--no-filename", "--no-messages",
        "--null", "--null-data", "--only-matching", "--perl-regexp", "--quiet",
        "--recursive", "--silent", "--text", "--with-filename", "--word-regexp",
        "-E", "-F", "-G", "-H", "-I", "-J", "-L", "-P", "-R", "-S",
        "-T", "-U", "-V", "-Z",
        "-a", "-b", "-c", "-h", "-i", "-l", "-n", "-o", "-p", "-q",
        "-r", "-s", "-v", "-w", "-x", "-z",
    ]),
    standalone_short: b"EFGHIJLPRSTUVZabchilnopqrsvwxz",
    valued: WordSet::new(&[
        "--after-context", "--before-context", "--binary-files", "--color",
        "--colour", "--context", "--devices", "--directories", "--exclude",
        "--exclude-dir", "--exclude-from", "--file", "--group-separator",
        "--include", "--label", "--max-count", "--regexp",
        "-A", "-B", "-C", "-D", "-d", "-e", "-f", "-m",
    ]),
    valued_short: b"ABCDdefm",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "egrep", policy: &GREP_POLICY, help_eligible: false },
    FlatDef { name: "fgrep", policy: &GREP_POLICY, help_eligible: false },
    FlatDef { name: "grep", policy: &GREP_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        grep_pattern: "grep pattern file.txt",
        grep_recursive: "grep -rn pattern .",
        grep_combined: "grep -inl pattern .",
        grep_context: "grep -A 3 -B 3 pattern file",
        grep_extended: "grep -E 'foo|bar' file",
        grep_fixed: "grep -F exact file",
        grep_count: "grep -c pattern file",
        grep_file_pattern: "grep -f patterns.txt file",
        grep_exclude: "grep --exclude='*.o' pattern .",
        grep_color: "grep --color pattern file",
        grep_color_eq: "grep --color=always pattern file",
        grep_max_count: "grep --max-count=5 pattern file",
        grep_null: "grep --null -l pattern .",
        grep_perl: "grep -P '\\d+' file",
        egrep_safe: "egrep 'foo|bar' file",
        fgrep_safe: "fgrep exact file",
    }
}
