use crate::parse::Token;
use crate::registry;
use crate::verdict::Verdict;

pub(crate) fn is_safe_sysctl(tokens: &[Token]) -> Verdict {
    if tokens[1..].iter().any(|t| t.contains('=')) {
        return Verdict::Denied;
    }
    registry::try_fallback_grammar("sysctl", tokens).unwrap_or(Verdict::Denied)
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        sysctl_help: "sysctl --help",
        sysctl_all: "sysctl -a",
        sysctl_specific: "sysctl kern.ostype",
        sysctl_n: "sysctl -n kern.ostype",
        sysctl_with_b_value: "sysctl -B 100 kern.ostype",
        sysctl_no_typecast: "sysctl -X",
        sysctl_combined_flags: "sysctl -an",
    }

    denied! {
        sysctl_bare: "sysctl",
        sysctl_write_eq: "sysctl kern.ostype=Darwin",
        sysctl_write_dash_w: "sysctl -w kern.ostype=Darwin",
        sysctl_load_p: "sysctl -p /etc/sysctl.conf",
        sysctl_system: "sysctl --system",
        sysctl_unknown_flag: "sysctl --evil",
        sysctl_value_in_eq_form: "sysctl -B=100",
    }

    use super::{Token, Verdict, is_safe_sysctl};
    use proptest::prelude::*;

    // Property tests: invariants of `is_safe_sysctl`.
    //
    // `is_safe_sysctl` is a three-line guard: it short-circuits to `Denied` the moment any non-first
    // token contains `=`, and otherwise delegates to the fallback grammar registered in
    // `commands/system/sysctl.toml`. Every property below pins one of those two behaviors so a
    // regression in either branch fails here.
    proptest! {
        // Invariant: the equals check is the FIRST thing the handler does — any non-first token
        // carrying `=` forces `Denied`, even when it follows a benign read-only prefix like `-a`.
        // Picked regex ensures the generated token always contains at least one `=`.
        #[test]
        fn sysctl_with_equals_in_any_tail_token_is_denied(
            prefix in prop_oneof![Just(String::new()), Just("-a".to_string()), Just("-n".to_string()), Just("--help".to_string())],
            key in "[a-zA-Z][a-zA-Z0-9._]{0,15}",
            eq_tok in "[a-zA-Z][a-zA-Z0-9._]{0,7}=[a-zA-Z0-9._-]{1,8}",
        ) {
            // `sysctl` may be the only token, the `eq_tok` alone, or `eq_tok` sandwiched between a
            // benign prefix and a benign key — every shape must deny.
            let line = if prefix.is_empty() {
                format!("sysctl {eq_tok}")
            } else {
                format!("sysctl {prefix} {eq_tok} {key}")
            };
            proptest::prop_assert!(
                !is_safe_command(&line),
                "sysctl with `=` in a tail token was allowed: `{}`",
                line,
            );
        }

        // Oracle: when NO non-first token carries `=`, the handler is exactly
        // `try_fallback_grammar("sysctl", tokens).unwrap_or(Verdict::Denied)`. Pinning that
        // equivalence means the equals-check is the ONLY branch the handler adds on top of the
        // grammar — a regression that introduced new logic here fails it.
        #[test]
        fn sysctl_without_equals_falls_through_to_fallback_grammar(
            // A handful of representative shapes that exercise both `bare=false` (the literal
            // `sysctl`) and the read paths (a flag, a flag with a value, a bare key).
            shape in prop_oneof![
                Just(vec!["sysctl"]),
                Just(vec!["sysctl", "-a"]),
                Just(vec!["sysctl", "-n", "kern.ostype"]),
                Just(vec!["sysctl", "-B", "100", "kern.ostype"]),
                Just(vec!["sysctl", "-q", "kern.ostype"]),
                Just(vec!["sysctl", "kern.ostype"]),
                Just(vec!["sysctl", "-x", "kern.ostype"]),
            ],
        ) {
            let tokens: Vec<Token> = shape.iter().map(|s| Token::from_test(s)).collect();
            proptest::prop_assert_eq!(
                is_safe_sysctl(&tokens),
                crate::registry::try_fallback_grammar("sysctl", &tokens).unwrap_or(Verdict::Denied),
                "without `=`, is_safe_sysctl diverged from the fallback grammar for: {:?}",
                tokens,
            );
        }

        // Metamorphic: `-q` is in the standalone-flag list, so it must NOT change the verdict
        // versus the same form without it. Comparing equality (and not just `is_allowed`) means
        // the property holds on every input — a regression that flipped allowance, or swapped an
        // `Inert` verdict for `SafeRead`, fails here.
        #[test]
        fn sysctl_safe_standalone_flag_does_not_change_verdict(
            key in "[a-z][a-z0-9._]{0,15}",
        ) {
            let base = format!("sysctl {key}");
            let quiet = format!("sysctl -q {key}");
            proptest::prop_assert_eq!(
                is_safe_command(&base),
                is_safe_command(&quiet),
                "appending the safe standalone `-q` changed the verdict: base=`{}` quiet=`{}`",
                base,
                quiet,
            );
        }
    }
}
