use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

fn is_safe_nslookup(tokens: &[Token]) -> Verdict {
    if tokens.len() == 2 && (tokens[1] == "--help" || tokens[1] == "-h" || tokens[1] == "--version" || tokens[1] == "-V") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    for t in &tokens[1..] {
        let s = t.as_str();
        if !s.starts_with('-') {
            continue;
        }
        if s == "-debug" || s == "-nodebug" || s == "-d2" {
            continue;
        }
        if s.starts_with("-type=")
            || s.starts_with("-query=")
            || s.starts_with("-port=")
            || s.starts_with("-timeout=")
            || s.starts_with("-retry=")
            || s.starts_with("-class=")
            || s.starts_with("-domain=")
            || s.starts_with("-querytype=")
        {
            continue;
        }
        return Verdict::Denied;
    }
    Verdict::Allowed(SafetyLevel::Inert)
}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "nslookup" => Some(is_safe_nslookup(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![crate::docs::CommandDoc::handler(
        "nslookup",
        "https://man7.org/linux/man-pages/man1/nslookup.1.html",
        "Allowed: positional args, -debug, -nodebug, -d2, and valued options (-type=, -query=, -port=, -timeout=, -retry=, -class=, -domain=, -querytype=).",
        "net",
    )]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] =
    &[crate::handlers::CommandEntry::Custom { cmd: "nslookup", valid_prefix: Some("nslookup example.com") }];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        nslookup_domain: "nslookup example.com",
        nslookup_server: "nslookup example.com 8.8.8.8",
        nslookup_type: "nslookup -type=MX example.com",
    }
}

#[cfg(test)]
mod proptests {
    use super::is_safe_nslookup;
    use crate::parse::Token;
    use crate::verdict::{SafetyLevel, Verdict};
    use proptest::prelude::*;

    /// A token that must be treated as a positional argument (never inspected by the function), built
    /// from arbitrary non-empty non-flag strings: bare words (`example.com`), dotted host-like strings
    /// (`mail.example.com`), IP-ish strings (`8.8.8.8`), and names with digits, dashes or underscores
    /// that start with something other than `-`.
    fn arb_positional() -> impl Strategy<Value = String> {
        prop_oneof!["[a-z][a-z0-9._-]{0,15}", "[a-z][a-z0-9-]{0,7}\\.[a-z]{2,5}", "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}",]
    }

    /// An arbitrary token that may be either a positional, a known-safe flag, a `-key=value` form for
    /// one of the eight valued options the function recognises, or a help/version short-circuit.
    /// Together these are exactly the forms the function must approve.
    fn arb_known_safe_token() -> impl Strategy<Value = String> {
        prop_oneof![
            arb_positional(),
            Just("-debug".to_string()),
            Just("-nodebug".to_string()),
            Just("-d2".to_string()),
            "-type=[A-Za-z]{1,6}",
            "-query=[a-z.]{1,12}",
            "-port=[0-9]{1,5}",
            "-timeout=[0-9]{1,3}",
            "-retry=[0-9]{1,2}",
            "-class=([a-z]{1,4}|[A-Z]{1,5})",
            "-domain=[a-z.-]{1,16}",
            "-querytype=[A-Za-z]{1,8}",
        ]
    }

    proptest! {
        /// Every nslookup command whose tokens after the program name are positionals, recognised
        /// debug flags, or one of the eight `-key=value` valued-option forms is approved as Inert.
        /// These are exactly the inputs the function's allow-branch must accept; a regression that
        /// starts denying any of them — for instance by narrowing the recognised prefixes — fails
        /// here.
        #[test]
        fn only_known_safe_tokens_yield_inert_verdict(
            tail in proptest::collection::vec(arb_known_safe_token(), 0..8),
        ) {
            let mut tokens: Vec<Token> = Vec::with_capacity(tail.len() + 1);
            tokens.push(Token::from_test("nslookup"));
            for t in &tail {
                tokens.push(Token::from_test(t));
            }
            let v = is_safe_nslookup(&tokens);
            prop_assert_eq!(v, Verdict::Allowed(SafetyLevel::Inert),
                "tail {:?} must be Inert, got {:?}", tail, v);
        }

        /// The function never returns a `SafeRead` or `SafeWrite` ceiling — it only ever distinguishes
        /// denied from `Allowed(Inert)`. A regression that promoted an allow-branch through
        /// `SafeWrite` (e.g. by copy-pasting from a write-capable handler) would surface here, because
        /// any lifted ceiling would mismatch the `Inert` invariant.
        #[test]
        fn every_allowed_verdict_is_exactly_inert(
            tail in proptest::collection::vec(any::<String>(), 0..6),
        ) {
            let mut tokens: Vec<Token> = Vec::with_capacity(tail.len() + 1);
            tokens.push(Token::from_test("nslookup"));
            for t in &tail {
                tokens.push(Token::from_test(t));
            }
            let v = is_safe_nslookup(&tokens);
            match v {
                Verdict::Denied => {}
                Verdict::Allowed(level) => prop_assert_eq!(level, SafetyLevel::Inert,
                    "nslookup must never lift above Inert; got {:?} for tail {:?}", level, tail),
            }
        }
    }
}
