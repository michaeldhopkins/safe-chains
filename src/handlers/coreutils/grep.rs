use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static GREP_STANDALONE: WordSet = WordSet::new(&[
    "--basic-regexp", "--binary", "--byte-offset", "--color", "--colour",
    "--count", "--dereference-recursive", "--extended-regexp",
    "--files-with-matches", "--files-without-match", "--fixed-strings",
    "--help", "--ignore-case", "--initial-tab", "--invert-match", "--line-buffered",
    "--line-number", "--line-regexp", "--no-filename", "--no-messages",
    "--null", "--null-data", "--only-matching", "--perl-regexp", "--quiet",
    "--recursive", "--silent", "--text", "--version", "--with-filename", "--word-regexp",
    "-E", "-F", "-G", "-H", "-I", "-J", "-L", "-P", "-R", "-S",
    "-T", "-U", "-V", "-Z",
    "-a", "-b", "-c", "-h", "-i", "-l", "-n", "-o", "-p", "-q",
    "-r", "-s", "-v", "-w", "-x", "-z",
]);

static GREP_VALUED: WordSet = WordSet::new(&[
    "--after-context", "--before-context", "--binary-files", "--color",
    "--colour", "--context", "--devices", "--directories", "--exclude",
    "--exclude-dir", "--exclude-from", "--file", "--group-separator",
    "--include", "--label", "--max-count", "--regexp",
    "-A", "-B", "-C", "-D", "-d", "-e", "-f", "-m",
]);

fn is_safe_grep(tokens: &[Token]) -> Verdict {
    if tokens.len() == 1 {
        return Verdict::Denied;
    }

    let mut i = 1;
    let mut has_pattern = false;
    while i < tokens.len() {
        let t = &tokens[i];

        if *t == "--" {
            return Verdict::Allowed(SafetyLevel::Inert);
        }

        if !t.starts_with('-') || has_pattern {
            has_pattern = true;
            i += 1;
            continue;
        }

        if GREP_STANDALONE.contains(t) {
            i += 1;
            continue;
        }

        if GREP_VALUED.contains(t) {
            i += 2;
            continue;
        }

        if let Some((flag, _)) = t.as_str().split_once('=')
            && GREP_VALUED.contains(flag)
        {
            i += 1;
            continue;
        }

        if t.starts_with("--") {
            has_pattern = true;
            i += 1;
            continue;
        }

        let bytes = t.as_bytes();
        let mut j = 1;
        let mut valid_combined = true;
        while j < bytes.len() {
            let b = bytes[j];
            let is_last = j == bytes.len() - 1;
            if GREP_STANDALONE.contains_short(b) {
                j += 1;
                continue;
            }
            if GREP_VALUED.contains_short(b) {
                if is_last {
                    i += 1;
                }
                break;
            }
            valid_combined = false;
            break;
        }
        if !valid_combined {
            return Verdict::Denied;
        }
        i += 1;
    }
    if has_pattern { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }
}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "grep" | "egrep" | "fgrep" => Some(is_safe_grep(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("grep / egrep / fgrep",
            "https://www.gnu.org/software/grep/manual/grep.html",
            "Pattern and file arguments accepted after flags. Patterns starting with - are allowed as positional arguments.",
            "search"),
    ]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Positional { cmd: "grep" },
    crate::handlers::CommandEntry::Positional { cmd: "egrep" },
    crate::handlers::CommandEntry::Positional { cmd: "fgrep" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        basic: "grep pattern file.txt",
        flags_before_pattern: "grep -rn pattern src/",
        combined_short_flags: "grep -rni pattern src/",
        regex: "grep -E '^[0-9]+' file",
        case_insensitive: "grep -i hello world.txt",
        count: "grep -c error log.txt",
        include: "grep -rn --include '*.rs' pattern src/",
        context: "grep -A 3 -B 2 error log.txt",
        regexp_flag: "grep -e pattern file.txt",
        regexp_long: "grep --regexp=pattern file.txt",
        multiple_e: "grep -e foo -e bar file.txt",
        file_flag: "grep -f patterns.txt file.txt",
        dash_dash_separator: "grep -- '-->' file.txt",
        pattern_looks_like_flag: "grep '-->' file.txt",
        pattern_with_dashes: "grep '--some-pattern' file.txt",
        pattern_triple_dash: "grep '---' file.txt",
        color_eq: "grep --color=always pattern file.txt",
        max_count: "grep -m 5 pattern file.txt",
        quiet: "grep -q pattern file.txt",
        invert: "grep -v pattern file.txt",
        pattern_only: "grep pattern",
        multiple_files: "grep pattern file1.txt file2.txt file3.txt",
        recursive: "grep -r pattern .",
        line_regexp: "grep -x 'exact match' file.txt",
        egrep: "egrep 'foo|bar' file.txt",
        fgrep: "fgrep 'literal string' file.txt",
        exclude_dir: "grep -r --exclude-dir=node_modules pattern .",
        null_flag: "grep -Z pattern file.txt",
        combined_with_valued: "grep -rnA3 pattern src/",
        pattern_starts_with_dash: "grep -rn '-->' src/",
        pattern_after_all_flags: "grep -i -r -n '-->' src/",
    }

    denied! {
        unknown_short_flag: "grep -Q pattern",
        bare: "grep",
        unknown_combined: "grep -rnQ pattern",
    }
}
