use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static ROUTE_SAFE_FLAGS: WordSet = WordSet::new(&["--help", "--version", "-4", "-6", "-V", "-h", "-n", "-v"]);
static ROUTE_SAFE_SUBCMDS: WordSet = WordSet::new(&["get", "monitor", "print", "show"]);

fn is_safe_route(tokens: &[Token]) -> Verdict {
    let mut i = 1;
    while i < tokens.len() {
        let t = &tokens[i];
        if ROUTE_SAFE_FLAGS.contains(t) {
            i += 1;
            continue;
        }
        if ROUTE_SAFE_SUBCMDS.contains(t) {
            return Verdict::Allowed(SafetyLevel::Inert);
        }
        return Verdict::Denied;
    }
    Verdict::Allowed(SafetyLevel::Inert)
}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "route" => Some(is_safe_route(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![crate::docs::CommandDoc::handler(
        "route",
        "https://man7.org/linux/man-pages/man8/route.8.html",
        "- Allowed subcommands: get, monitor, print, show\n- Allowed flags: -4, -6, -n, -v\n- Bare invocation allowed",
        "net",
    )]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        route_bare: "route",
        route_n: "route -n",
        route_print: "route print",
        route_get: "route get 8.8.8.8",
        route_show: "route show",
        route_monitor: "route monitor",
        route_dash_v: "route -v",
        route_n_get: "route -n get 8.8.8.8",
        route_4_get: "route -4 get 8.8.8.8",
    }

    denied! {
        route_add_denied: "route add default 192.168.1.1",
        route_del_denied: "route del default",
        route_delete_denied: "route delete default",
        route_change_denied: "route change default 192.168.1.1",
        route_flush_denied: "route flush",
        route_replace_denied: "route replace default via 192.168.1.1",
        route_n_add_denied: "route -n add default 192.168.1.1",
    }

    use proptest::prelude::*;

    /// Every safe flag this handler whitelists (mirrors `ROUTE_SAFE_FLAGS`), for sampling in
    /// property tests. Keeping it local means the property fails LOUDLY if a future change to the
    /// allowlist is missed here — the proptest would suddenly over- or under-restrict.
    const SAFE_FLAGS: &[&str] = &["--help", "--version", "-4", "-6", "-V", "-h", "-n", "-v"];
    /// Every safe subcommand this handler whitelists (mirrors `ROUTE_SAFE_SUBCMDS`).
    const SAFE_SUBCMDS: &[&str] = &["get", "monitor", "print", "show"];

    proptest! {
        /// The classifier behaves exactly like a reference implementation written straight from the
        /// policy statement: scan past any leading flag tokens from the whitelist; if the next token
        /// is a subcommand from the whitelist (or there is no next token), allow; otherwise deny.
        /// A regression that drops a flag from the walk, returns from a flag (instead of continuing),
        /// or fails to short-circuit on a subcommand fails here — the reference and the function
        /// would diverge on the constructed inputs.
        #[test]
        fn is_safe_route_matches_the_specified_reference(
            flags in proptest::collection::vec(proptest::sample::select(SAFE_FLAGS), 0..8),
            extra in proptest::collection::vec("[a-zA-Z0-9._:/-]{0,6}", 0..6),
        ) {
            let mut tokens: Vec<crate::parse::Token> = Vec::with_capacity(1 + flags.len() + 1 + extra.len());
            tokens.push(crate::parse::Token::from_raw("route".to_string()));
            for f in &flags {
                tokens.push(crate::parse::Token::from_raw((*f).to_string()));
            }
            for e in &extra {
                tokens.push(crate::parse::Token::from_raw(e.clone()));
            }

            let actual = super::is_safe_route(&tokens);

            // Reference: walk past safe flags; the FIRST token that is not a flag decides.
            // If we walk off the end → allowed; if the deciding token is a safe subcommand →
            // allowed; otherwise denied.
            let mut allowed = true;
            let mut decided = false;
            for t in tokens.iter().skip(1) {
                if SAFE_FLAGS.contains(&t.as_str()) { continue; }
                allowed = SAFE_SUBCMDS.contains(&t.as_str());
                decided = true;
                break;
            }
            let reference = if decided { if allowed { super::Verdict::Allowed(super::SafetyLevel::Inert) } else { super::Verdict::Denied } } else { super::Verdict::Allowed(super::SafetyLevel::Inert) };

            prop_assert_eq!(actual, reference, "flags={:?} extra={:?}", flags, extra);
        }

        /// Once the first non-flag token is a recognized subcommand, the verdict is fixed at
        /// `Allowed(Inert)` no matter what comes next — the function returns immediately at the
        /// subcommand. Appending random garbage (including tokens that look like dangerous
        /// subcommands) cannot flip the verdict; only moving the subcommand elsewhere can. This is
        /// the policy itself: route's mutating verbs are rejected because they appear BEFORE a
        /// recognized subcommand; trailing tokens are inert.
        #[test]
        fn trailing_tokens_after_a_safe_subcommand_are_inert(
            flags in proptest::collection::vec(proptest::sample::select(SAFE_FLAGS), 0..6),
            sub in proptest::sample::select(SAFE_SUBCMDS),
            suffix in proptest::collection::vec("[a-zA-Z0-9._:/-]{1,8}", 0..10),
        ) {
            let mut tokens: Vec<crate::parse::Token> = Vec::with_capacity(1 + flags.len() + 1 + suffix.len());
            tokens.push(crate::parse::Token::from_raw("route".to_string()));
            for f in &flags {
                tokens.push(crate::parse::Token::from_raw((*f).to_string()));
            }
            tokens.push(crate::parse::Token::from_raw((*sub).to_string()));
            for s in &suffix {
                tokens.push(crate::parse::Token::from_raw(s.clone()));
            }

            prop_assert_eq!(
                super::is_safe_route(&tokens),
                super::Verdict::Allowed(super::SafetyLevel::Inert),
                "flags={:?} sub={} suffix={:?}",
                flags, sub, suffix,
            );
        }

        /// The first non-flag token that is not a recognized subcommand DENIES — independent of any
        /// safe flag prefix, independent of any safe subcommand that appears LATER (the function
        /// never reaches it). This pins the negative side of the policy: a mutating verb
        /// (`add`/`del`/`change`/`flush`/`replace`) anywhere before position `k` cannot be
        /// "rescued" by a later `print`. A regression that delayed the deny until the END of the
        /// token list would fail here.
        #[test]
        fn any_unknown_token_before_any_subcommand_denies(
            flags in proptest::collection::vec(proptest::sample::select(SAFE_FLAGS), 0..6),
            hostile in proptest::sample::select(&["add", "del", "delete", "change", "flush", "replace", "evil", "set", "unset", "frob"][..]),
            more in proptest::collection::vec(proptest::sample::select(SAFE_SUBCMDS), 0..3),
        ) {
            let mut tokens: Vec<crate::parse::Token> = Vec::with_capacity(1 + flags.len() + 1 + more.len());
            tokens.push(crate::parse::Token::from_raw("route".to_string()));
            for f in &flags {
                tokens.push(crate::parse::Token::from_raw((*f).to_string()));
            }
            tokens.push(crate::parse::Token::from_raw((*hostile).to_string()));
            for m in &more {
                tokens.push(crate::parse::Token::from_raw((*m).to_string()));
            }

            prop_assert_eq!(
                super::is_safe_route(&tokens),
                super::Verdict::Denied,
                "flags={:?} hostile={} later_subs={:?}",
                flags, hostile, more,
            );
        }

        /// `command_docs()` is the only documentation this handler publishes; it must describe
        /// exactly the `route` command, in the `net` category, with a reachable https URL and a
        /// non-empty body. A regression that accidentally added a second entry, dropped the URL,
        /// renamed the command or moved it out of `net` would silently break the generated
        /// `--list-commands` output, and the rest of the suite does not check it (the global docs
        /// check only asserts every listed command has *some* doc, not that this handler has
        /// exactly one).
        #[test]
        fn command_docs_is_a_single_route_entry(
            _ in proptest::num::u8::ANY,
        ) {
            let docs = super::command_docs();
            prop_assert_eq!(docs.len(), 1, "expected exactly one CommandDoc entry, got {}", docs.len());
            let doc = &docs[0];
            prop_assert_eq!(&doc.name, "route");
            prop_assert_eq!(&doc.category, "net");
            prop_assert!(!doc.url.is_empty(), "url must be non-empty");
            prop_assert!(doc.url.starts_with("https://"), "url must be https, got {}", doc.url);
            prop_assert!(!doc.description.is_empty(), "description must be non-empty");
            prop_assert!(doc.aliases.is_empty(), "aliases must be empty");
            prop_assert!(doc.examples.is_empty(), "examples must be empty");
        }
    }
}
