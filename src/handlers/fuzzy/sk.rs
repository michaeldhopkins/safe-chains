use crate::parse::{Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagTolerance};
use crate::verdict::{SafetyLevel, Verdict};

static SK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--ansi", "--border", "--cycle", "--exact", "--exit-0",
        "--help", "--inline-info", "--keep-right", "--layout",
        "--multi", "--no-bold", "--no-clear", "--no-hscroll", "--no-mouse",
        "--no-sort", "--print-query", "--print-score", "--print0",
        "--read0", "--regex", "--reverse", "--select-1",
        "--tac", "--version",
        "-0", "-1", "-V", "-e", "-h", "-m",
    ]),
    valued: WordSet::flags(&[
        "--algo", "--border-label", "--color", "--delimiter",
        "--expect", "--header", "--header-lines", "--height",
        "--history", "--history-size", "--margin", "--min-height",
        "--nth", "--output-format", "--padding",
        "--pre-select-file", "--pre-select-items", "--pre-select-n",
        "--prompt", "--query", "--scrollbar", "--skip-to-pattern",
        "--tabstop", "--tiebreak", "--with-nth",
        "-d", "-n", "-p", "-q",
    ]),
    bare: true,
    max_positional: None,
    tolerance: FlagTolerance::strict(),
};

static HISTORY_FLAGS: WordSet = WordSet::flags(&["--history"]);

fn has_history(tokens: &[Token]) -> bool {
    tokens[1..].iter().any(|t| {
        HISTORY_FLAGS.contains(t)
            || t.as_str().starts_with("--history=")
    })
}

pub fn is_safe_sk(tokens: &[Token]) -> Verdict {
    if !policy::check(tokens, &SK_POLICY) {
        return Verdict::Denied;
    }
    if tokens.len() > 1 && has_history(tokens) {
        Verdict::Allowed(SafetyLevel::SafeWrite)
    } else {
        Verdict::Allowed(SafetyLevel::Inert)
    }
}

pub(super) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "sk" => Some(is_safe_sk(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler(
            "sk",
            "https://github.com/lotabout/skim",
            "Fuzzy finder. Display, filtering, and layout flags allowed. --history allowed (SafeWrite).",
            "fuzzy",
        ),
    ]
}

#[cfg(test)]
pub(super) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "sk", valid_prefix: None },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    use crate::command_verdict;
    use crate::parse::Token;
    use crate::verdict::{SafetyLevel, Verdict};
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        sk_bare: "sk",
        sk_help: "sk --help",
        sk_version: "sk --version",
        sk_multi: "sk -m",
        sk_exact: "sk --exact",
        sk_query: "sk -q pattern",
        sk_height: "sk --height 40%",
        sk_ansi: "sk --ansi",
        sk_prompt: "sk --prompt '> '",
        sk_header: "sk --header 'Pick one'",
        sk_color: "sk --color=dark",
        sk_delimiter: "sk -d : -n 2",
        sk_select_1: "sk --select-1 --exit-0",
        sk_history: "sk --history /tmp/sk-history",
        sk_history_eq: "sk --history=/tmp/sk-history",
    }

    safe_write! {
        sk_history_level: "sk --history /tmp/sk-history",
    }

    denied! {
        sk_preview: "sk --preview 'cat {}'",
        sk_cmd: "sk -c 'find .'",
        sk_interactive: "sk -i -c 'grep {}'",
        sk_bind: "sk --bind 'ctrl-j:accept'",
    }

    #[test]
    fn sk_bare_is_inert() {
        assert_eq!(command_verdict("sk"), Verdict::Allowed(SafetyLevel::Inert));
    }

    use proptest::prelude::*;
    proptest! {
        /// A history block at the tail of a token list that already passes the SK policy must produce
        /// exactly `Allowed(SafeWrite)` — the history flag is the ONE clause that lifts the verdict from
        /// `Inert` to `SafeWrite`, and it can only fire when the surrounding policy already accepts the
        /// invocation. The varied corpus of leading standalone/valued flags is the only way to confirm
        /// that no other flag happens to coincide with `--history`'s effect: a hand-picked example could
        /// silently get the right answer for the wrong reason.
        #[test]
        fn a_history_block_within_a_passing_policy_yields_exactly_safe_write(
            lead in proptest::collection::vec(
                proptest::sample::select(vec![
                    "--ansi", "--border", "--cycle", "--exact", "--exit-0", "--help",
                    "--inline-info", "--keep-right", "--layout", "--multi", "--no-bold",
                    "--no-clear", "--no-hscroll", "--no-mouse", "--no-sort", "--print-query",
                    "--print-score", "--print0", "--read0", "--regex", "--reverse",
                    "--select-1", "--tac", "--version", "-0", "-1", "-V", "-e", "-h", "-m",
                    "--algo=skim", "--border-label=top", "--color=dark", "--delimiter=:",
                    "--expect=ctrl-j", "--header=Pick", "--header-lines=2", "--height=40%",
                    "--history=/tmp/x", "--history-size=100", "--margin=1", "--min-height=5",
                    "--nth=1", "--output-format=json", "--padding=1", "--pre-select-file=.",
                    "--pre-select-items=1", "--pre-select-n=1", "--prompt=>", "--query=p",
                    "--scrollbar=", "--skip-to-pattern=", "--tabstop=8", "--tiebreak=score",
                    "--with-nth=1", "-d::", "-n2", "-pp", "-q p",
                ]),
                0..=8,
            ),
            history_path in "[A-Za-z0-9_./]{1,16}",
        ) {
            let mut tokens: Vec<Token> = Vec::with_capacity(lead.len() + 3);
            tokens.push(Token::from_test("sk"));
            for flag in &lead {
                tokens.push(Token::from_test(flag));
            }
            tokens.push(Token::from_test("--history"));
            tokens.push(Token::from_test(&history_path));

            prop_assert!(
                super::policy::check(&tokens, &super::SK_POLICY),
                "constructed invocation should pass SK_POLICY: {:?}",
                tokens.iter().map(|t| t.as_str()).collect::<Vec<_>>()
            );
            prop_assert_eq!(
                super::is_safe_sk(&tokens),
                Verdict::Allowed(SafetyLevel::SafeWrite),
                "history block in a passing invocation must yield SafeWrite"
            );
        }

        /// The history flag may upgrade an inert invocation to a write-allowing one, but it CANNOT
        /// rescue an invocation that the policy itself has refused. A token set that the policy rejects
        /// must stay denied no matter where the history block sits, otherwise a deny-rule regression
        /// would silently turn into a SafeWrite hole the moment a future user appends `--history`.
        #[test]
        fn a_policy_failure_yields_denied_with_or_without_history(
            bad_flag in proptest::sample::select(vec![
                "--evil", "-Z", "--execute", "-x", "--bogus=1", "-r./bad",
            ]),
            history_value in "[A-Za-z0-9_./]{1,12}",
            history_first in proptest::bool::ANY,
        ) {
            // Build `["sk", <bad>, "--history", <path>]` in one order, then swap into the other. The
            // flag walk processes tokens left-to-right, so position is part of what we mean by "everywhere".
            let mut tokens: Vec<Token> = Vec::with_capacity(4);
            tokens.push(Token::from_test("sk"));
            if history_first {
                tokens.push(Token::from_test("--history"));
                tokens.push(Token::from_test(&history_value));
                tokens.push(Token::from_test(bad_flag));
            } else {
                tokens.push(Token::from_test(bad_flag));
                tokens.push(Token::from_test("--history"));
                tokens.push(Token::from_test(&history_value));
            }

            prop_assert!(
                !super::policy::check(&tokens, &super::SK_POLICY),
                "constructed invocation should FAIL SK_POLICY ({}): {:?}",
                bad_flag,
                tokens.iter().map(|t| t.as_str()).collect::<Vec<_>>()
            );
            prop_assert_eq!(
                super::is_safe_sk(&tokens),
                Verdict::Denied,
                "history must not rescue a failing policy (history_first={})",
                history_first
            );
        }

        /// `has_history` walks `tokens[1..]` looking for an entry that equals `--history` or starts
        /// with `--history=`. A token that matches NEITHER pattern therefore cannot change the
        /// answer, no matter what else is already in the slice — appending it is the identity on
        /// `has_history`. The pre-existing slice is varied so that no incidental match (e.g. a token
        /// already named `--history=…`) slips through unnoticed.
        #[test]
        fn appending_a_non_history_token_keeps_has_history(
            base in proptest::collection::vec(any::<String>(), 1..=6),
            appended in "[A-Za-z0-9_./+_-]{1,12}",
        ) {
            prop_assume!(!base.is_empty(), "the leading command token is required");
            let tokens_before: Vec<Token> = base.iter().map(|s| Token::from_test(s)).collect();

            // A history-mention in `base` already locks the answer to true; a non-history mention keeps
            // it the same. The appended string is constrained to NOT be the two matching shapes, so the
            // property claims: the answer is unchanged.
            let token = appended.clone();
            prop_assume!(token != "--history", "appended token must not match the bare form");
            prop_assume!(
                !token.starts_with("--history="),
                "appended token must not match the glued form"
            );

            let mut tokens_after = tokens_before.clone();
            tokens_after.push(Token::from_test(&token));

            let before = super::has_history(&tokens_before);
            let after = super::has_history(&tokens_after);
            prop_assert_eq!(
                before, after,
                "appending a non-history token flipped has_history: before={}, after={}, appended={:?}",
                before, after, token
            );
        }
    }
}
