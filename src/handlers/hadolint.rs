//! `hadolint` is a GHC program, so a build linked with `-rtsopts` lets `+RTS -S<file>` or
//! `-t<file>` write GC statistics to a named file. Any runtime-option marker is refused; the flag
//! lists in `commands/containers/hadolint.toml` decide everything else.
use crate::parse::Token;
use crate::registry;
use crate::verdict::Verdict;

pub fn check_hadolint(tokens: &[Token]) -> Verdict {
    if tokens.iter().any(|t| super::pandoc::is_runtime_option(t.as_str())) {
        return Verdict::Denied;
    }
    registry::try_fallback_grammar("hadolint", tokens).unwrap_or(Verdict::Denied)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    fn check(cmd: &str) -> bool {
        crate::is_safe_command(cmd)
    }
    use crate::verdict::{SafetyLevel, Verdict};

    safe! {
        plain_dockerfile: "hadolint Dockerfile",
        json_report: "hadolint --no-fail -f json Dockerfile",
        unknown_short_tolerated: "hadolint -z Dockerfile",
    }

    denied! {
        stats_file: "hadolint Dockerfile +RTS -Sx",
        first: "hadolint +RTS -tpwned Dockerfile",
        closed: "hadolint Dockerfile +RTS -s -RTS",
        glued_file: "hadolint +RTS -sout -RTS Dockerfile",
        end_marker: "hadolint --RTS Dockerfile",
        bare_end: "hadolint Dockerfile -RTS",
        bare: "hadolint",
    }

    #[test]
    fn a_plain_dockerfile_is_safe_read() {
        assert_eq!(crate::command_verdict("hadolint Dockerfile"), Verdict::Allowed(SafetyLevel::SafeRead));
    }

    proptest! {
        #[test]
        fn any_runtime_option_token_is_denied(
            before in "[a-zA-Z.]{0,6}", after in "[a-zA-Z0-9.-]{0,8}",
            marker in prop::sample::select(vec!["+RTS", "-RTS", "--RTS", "+RTSx"]),
        ) {
            let cmd = format!("hadolint {before} {marker} {after}");
            prop_assert!(!crate::is_safe_command(&cmd), "{}", cmd);
        }
    }
}
