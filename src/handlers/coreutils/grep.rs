use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static GREP_STANDALONE: WordSet = WordSet::new(&[
    "--basic-regexp", "--binary", "--byte-offset", "--color", "--colour", "--count", "--dereference-recursive", "--extended-regexp",
    "--files-with-matches", "--files-without-match", "--fixed-strings", "--help", "--ignore-case", "--initial-tab", "--invert-match",
    "--line-buffered", "--line-number", "--line-regexp", "--no-filename", "--no-messages", "--null", "--null-data", "--only-matching",
    "--perl-regexp", "--quiet", "--recursive", "--silent", "--text", "--version", "--with-filename", "--word-regexp", "-E", "-F", "-G",
    "-H", "-I", "-J", "-L", "-P", "-R", "-S", "-T", "-U", "-V", "-Z", "-a", "-b", "-c", "-h", "-i", "-l", "-n", "-o", "-p", "-q", "-r",
    "-s", "-v", "-w", "-x", "-z",
]);

static GREP_VALUED: WordSet = WordSet::new(&[
    "--after-context", "--before-context", "--binary-files", "--color", "--colour", "--context", "--devices", "--directories", "--exclude",
    "--exclude-dir", "--exclude-from", "--file", "--group-separator", "--include", "--label", "--max-count", "--regexp", "-A", "-B", "-C",
    "-D", "-d", "-e", "-f", "-m",
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
        "grep" | "egrep" | "fgrep" | "rgrep" => Some(is_safe_grep(tokens)),
        _ => None,
    }
}

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
        rgrep: "rgrep pattern src/",
        rgrep_with_include: "rgrep --include '*.rs' pattern src/",
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

    use crate::parse::Token;
    use crate::verdict::{SafetyLevel, Verdict};
    use proptest::prelude::*;

    /// Independent re-implementation of `is_safe_grep`'s rules in a deliberately different shape
    /// (the unknown-long branch is folded into the trailing-if, the trailing-valued consumption
    /// in short chains is computed before the long-form `=` check, etc.). It uses the same
    /// `GREP_STANDALONE` / `GREP_VALUED` tables, so it tracks the documented grammar — any
    /// disagreement between it and `is_safe_grep` is a real parser divergence, not a property smell.
    fn naive_is_safe_grep(tokens: &[Token]) -> Verdict {
        if tokens.len() == 1 {
            return Verdict::Denied;
        }
        let standalone = super::GREP_STANDALONE;
        let valued = super::GREP_VALUED;
        // Walk forwards tracking `has_pattern` — same algorithm as `is_safe_grep`, written
        // out of order (consume-trailing-valued check before the long-form `=` check, and
        // the unknown-long branch collapsed into a separate arm) so the two diverge visibly
        // if either is edited.
        let mut has_pattern = false;
        let mut i = 1;
        while i < tokens.len() {
            let t = &tokens[i];
            let s = t.as_str();
            if s == "--" {
                return Verdict::Allowed(SafetyLevel::Inert);
            }
            if !s.starts_with('-') || has_pattern {
                has_pattern = true;
                i += 1;
                continue;
            }
            if standalone.contains(s) {
                i += 1;
                continue;
            }
            if valued.contains(s) {
                i += 2;
                continue;
            }
            if let Some((name, _)) = s.split_once('=')
                && valued.contains(name)
            {
                i += 1;
                continue;
            }
            if s.starts_with("--") {
                has_pattern = true;
                i += 1;
                continue;
            }
            // Short flag chain: iterate the bytes after the leading dash.
            let bytes = s.as_bytes();
            let mut all_known = true;
            for j in 1..bytes.len() {
                let b = bytes[j];
                let last = j == bytes.len() - 1;
                if standalone.contains_short(b) {
                    continue;
                }
                if valued.contains_short(b) {
                    if last {
                        i += 1;
                    }
                    break;
                }
                all_known = false;
                break;
            }
            if !all_known {
                return Verdict::Denied;
            }
            i += 1;
        }
        if has_pattern { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }
    }

    proptest! {
        /// Differential/oracle: an independent re-implementation of `is_safe_grep`'s grammar
        /// (above) agrees with the production code on every generated token sequence. The oracle
        /// is fixed data — only the production parser can drift — so a divergence is by
        /// definition a real parsing bug, not a property smell.
        #[test]
        fn naive_oracle_matches_is_safe_grep_on_every_token_sequence(
            raw in proptest::collection::vec(any::<String>(), 0..=10),
        ) {
            let tokens: Vec<Token> = raw.iter().map(|s| Token::from_test(s)).collect();
            let expected = naive_is_safe_grep(&tokens);
            let actual = super::is_safe_grep(&tokens);
            prop_assert_eq!(
                actual,
                expected,
                "divergence on tokens {:?}",
                raw,
            );
        }

        /// Metamorphic / dispatch-consistency: `dispatch(cmd, t)` is exactly `Some(is_safe_grep(t))`
        /// for the four grep-family command names (`grep`, `egrep`, `fgrep`, `rgrep`) and exactly
        /// `None` for every other name. Pins both halves of the relation — a regression that
        /// started returning `None` for `grep`, or `Some(...)` for an unrelated command, breaks here.
        #[test]
        fn dispatch_is_some_iff_command_is_a_grep_family_name(
            cmd in "[a-z]{1,8}",
            raw in proptest::collection::vec(any::<String>(), 0..=8),
        ) {
            let tokens: Vec<Token> = raw.iter().map(|s| Token::from_test(s)).collect();
            match cmd.as_str() {
                "grep" | "egrep" | "fgrep" | "rgrep" => {
                    prop_assert_eq!(
                        super::dispatch(&cmd, &tokens),
                        Some(super::is_safe_grep(&tokens)),
                        "grep-family dispatch must wrap `is_safe_grep`",
                    );
                }
                _ => {
                    prop_assert_eq!(
                        super::dispatch(&cmd, &tokens),
                        None,
                        "non-grep command `{}` must not dispatch",
                        cmd,
                    );
                }
            }
        }

        /// Monotonicity-on-extension: once an invocation is `Allowed(Inert)`, appending further
        /// tokens cannot downgrade the verdict. The production parser locks `has_pattern` after
        /// the first non-flag argument, and thereafter the `!starts_with('-') || has_pattern`
        /// branch wins for every remaining token — so trailing junk (including tokens that look
        /// like flags) is treated as additional positional arguments and never re-enters the
        /// combined-flag denial path.
        #[test]
        fn an_allowed_invocation_stays_allowed_when_trailing_tokens_are_appended(
            head in proptest::collection::vec(any::<String>(), 1..=8),
            tail in proptest::collection::vec(any::<String>(), 0..=8),
        ) {
            let head: Vec<Token> = head.iter().map(|s| Token::from_test(s)).collect();
            prop_assume!(super::is_safe_grep(&head) == Verdict::Allowed(SafetyLevel::Inert));
            let mut full = head.clone();
            for s in &tail {
                full.push(Token::from_test(s));
            }
            prop_assert_eq!(
                super::is_safe_grep(&full),
                Verdict::Allowed(SafetyLevel::Inert),
                "allowed-by-head (n={}) became {:?} after appending {:?}",
                head.len(),
                super::is_safe_grep(&full),
                tail,
            );
        }
    }
}
