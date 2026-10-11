use crate::verdict::{SafetyLevel, Verdict};
use crate::parse::Token;

fn is_list_mode(arg: &str) -> bool {
    if arg == "--list" {
        return true;
    }
    if !arg.starts_with('-') || arg.starts_with("--") {
        return false;
    }
    let flags = &arg[1..];
    flags.contains('t') && !flags.contains('c') && !flags.contains('x') && !flags.contains('u')
}

fn no_flags_after(tokens: &[Token], start: usize) -> bool {
    tokens[start..].iter().all(|t| !t.starts_with('-'))
}

pub fn is_safe_jar(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let first = tokens[1].as_str();
    match first {
        "--version" | "--help" => tokens.len() == 2,
        "tf" | "tvf" => no_flags_after(tokens, 2),
        _ => is_list_mode(first) && no_flags_after(tokens, 2),
    }
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    if cmd == "jar" {
        Some(if is_safe_jar(tokens) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied })
    } else {
        None
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("jar",
            "https://docs.oracle.com/en/java/javase/21/docs/specs/man/jar.html",
            "List mode only: tf, tvf, --list, -t. Also --version, --help.",
            "jvm"),
    ]
}

#[cfg(test)]
pub(in crate::handlers::jvm) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "jar", valid_prefix: Some("jar tf") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        jar_tf: "jar tf app.jar",
        jar_tvf: "jar tvf app.jar",
        jar_list: "jar --list",
        jar_t: "jar -tf app.jar",
        jar_tv: "jar -tvf app.jar",
        jar_version: "jar --version",
        jar_help: "jar --help",
    }

    denied! {
        jar_bare_denied: "jar",
        jar_create_denied: "jar cf app.jar .",
        jar_extract_denied: "jar xf app.jar",
        jar_update_denied: "jar uf app.jar file.class",
        jar_ct_denied: "jar -ctf app.jar",
        jar_unknown_denied: "jar --unknown",
    }

    /// Property tests for the list-mode selection (`is_list_mode`) and the tail predicate
    /// (`no_flags_after`) that `is_safe_jar` composes. The harness's allowed set for `jar` is built
    /// from these — every input that the function admits must trace through to one of the three
    /// branches of the match — so a regression in any of them can quietly widen or narrow the allow
    /// set. The properties below pin those branches against the simple reference behaviour spelled
    /// out as ordinary Rust expressions, so a future change has to keep the invariants, not just
    /// the existing snapshot of cases.
    use crate::parse::Token;
    use proptest::prelude::*;

    /// A short flag string: starts with a single `-` (NOT `--`), at least one flag character after.
    /// This is exactly the form `is_list_mode` examines past its early returns — `--list` and plain
    /// words are handled elsewhere.
    fn arb_short_flag() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("-t".to_string()),
            Just("-v".to_string()),
            "[a-zA-Z][a-zA-Z]{0,3}".prop_map(|f| format!("-{f}")),
        ]
    }

    /// A non-flag positional token: anything that doesn't start with `-`, so a path, a name, or a
    /// bare word. Mirrors the strings the tail predicate has to accept as "clean" (no flag) by
    /// construction, so that when we use it as a tail `no_flags_after` is forced to be true.
    fn arb_non_flag_token() -> impl Strategy<Value = String> {
        prop_oneof![
            "[a-zA-Z][a-zA-Z0-9._-]{0,15}",
            "[a-zA-Z][a-zA-Z0-9_-]{0,7}\\.[a-zA-Z]{2,5}",
        ]
    }

    proptest! {
        /// A short-flag string that contains any of `c`, `x`, or `u` is destructive (`cf`, `xf`,
        /// `uf`, mixes thereof); `is_list_mode` rejects ALL of them, regardless of whether `t` is
        /// also present. This is the load-bearing safety rule — the whole point of the handler is
        /// that short flags with those letters can't sneak in as list mode — so any regression
        /// that drops one of the negations allows a destructive flag through.
        #[test]
        fn a_short_flag_containing_a_forbidden_letter_is_not_list_mode(
            prefix in "[a-zA-Z]{0,4}",
            suffix in "[a-zA-Z]{0,4}",
            forbidden in prop_oneof![Just('c'), Just('x'), Just('u')],
        ) {
            // Build a flag like "-<prefix><forbidden><suffix>"; guaranteed single-`-`, not `--`,
            // and guaranteed to contain at least one forbidden letter.
            let flag = format!("-{prefix}{forbidden}{suffix}");
            prop_assert!(
                !super::is_list_mode(&flag),
                "forbidden-letter flag {:?} must not be list mode",
                flag,
            );
        }

        /// The long-flag namespace has exactly ONE list-mode string: `"--list"`. Every other
        /// `--`-prefixed token — including `"--listfoo"`, `"--listed"`, `"--listing"`, `"--"`,
        /// `"--version"`, `"--unknown"` — must be rejected. Pins the early `arg == "--list"` vs
        /// `arg.starts_with("--")` distinction: a regression that flipped either branch would
        /// start admitting variants like `"--listed"` or shut out `"--list"`.
        #[test]
        fn a_long_flag_is_list_mode_only_when_it_equals_dash_dash_list(
            flag in "--[a-zA-Z][a-zA-Z]*",
        ) {
            let expected = flag == "--list";
            prop_assert_eq!(
                super::is_list_mode(&flag),
                expected,
                "long flag {:?} list-mode should be {}",
                flag,
                expected,
            );
        }

        /// `is_safe_jar` is `is_list_mode(first) && no_flags_after(tokens, 2)` whenever the first
        /// arg is a short flag (or `tf`/`tvf`). With a tail built from non-flag tokens by
        /// construction, `no_flags_after` MUST be true, so the verdict reduces exactly to
        /// `is_list_mode(first)`. This is the composition contract — the handler allows precisely
        /// the list-mode flags and nothing else — captured against the simple reference formula
        /// rather than the case-by-case match arm.
        #[test]
        fn is_safe_jar_with_short_flag_matches_is_list_mode_when_tail_is_clean(
            flag in arb_short_flag(),
            tail in proptest::collection::vec(arb_non_flag_token(), 0..6),
        ) {
            let mut tokens: Vec<Token> = Vec::with_capacity(2 + tail.len());
            tokens.push(Token::from_test("jar"));
            tokens.push(Token::from_test(&flag));
            for t in &tail {
                tokens.push(Token::from_test(t));
            }
            prop_assert_eq!(
                super::is_safe_jar(&tokens),
                super::is_list_mode(&flag),
                "is_safe_jar must reduce to is_list_mode for short flag {:?} with clean tail",
                flag,
            );
        }

        /// `no_flags_after(tokens, start)` must agree with the naïve "scan from `start` and reject
        /// the first `-`-prefixed token" oracle. A regression that touches the index arithmetic
        /// (off-by-one between `start` and `start..`, mixing inclusive/exclusive) changes one and
        /// not the other. The `start == tokens.len()` arm of the property also locks down the
        /// empty-slice boundary — `no_flags_after` returns true for an empty range.
        #[test]
        fn no_flags_after_agrees_with_a_freshly_filtered_tail(
            tokens in proptest::collection::vec(
                prop_oneof![arb_non_flag_token(), "[a-zA-Z][a-zA-Z0-9_]{0,7}".prop_map(|f| format!("-{f}"))],
                0..10,
            ),
            start in 0..=10usize,
        ) {
            // Clamp `start` so the call stays within `0..=tokens.len()`.
            let start = start.min(tokens.len());
            let toks: Vec<Token> = tokens.iter().map(|s| Token::from_test(s)).collect();
            let expected = toks[start..].iter().all(|t| !t.starts_with('-'));
            prop_assert_eq!(
                super::no_flags_after(&toks, start),
                expected,
                "no_flags_after disagree at start={} for tokens={:?}",
                start,
                &toks,
            );
        }
    }
}
