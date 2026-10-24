//! `shellcheck` is a GHC program, so a build linked with `-rtsopts` lets `+RTS -s<file>` or
//! `-t<file>` write GC statistics to a named file. Any runtime-option marker is refused; the flag
//! lists in `commands/tools/shellcheck.toml` decide everything else.
use crate::parse::Token;
use crate::registry;
use crate::verdict::Verdict;

pub fn check_shellcheck(tokens: &[Token]) -> Verdict {
    if tokens.iter().any(|t| super::pandoc::is_runtime_option(t.as_str())) {
        return Verdict::Denied;
    }
    registry::try_fallback_grammar("shellcheck", tokens).unwrap_or(Verdict::Denied)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    fn check(cmd: &str) -> bool {
        crate::is_safe_command(cmd)
    }
    use crate::verdict::{SafetyLevel, Verdict};

    safe! {
        plain_script: "shellcheck script.sh",
        shell_and_external_sources: "shellcheck -x -s bash script.sh",
    }

    denied! {
        stats_file: "shellcheck script.sh +RTS -tpwned",
        first: "shellcheck +RTS -tpwned script.sh",
        closed: "shellcheck script.sh +RTS -s -RTS",
        glued_file: "shellcheck +RTS -sout -RTS script.sh",
        end_marker: "shellcheck --RTS script.sh",
        bare_end: "shellcheck script.sh -RTS",
    }

    #[test]
    fn a_plain_script_is_inert() {
        assert_eq!(crate::command_verdict("shellcheck script.sh"), Verdict::Allowed(SafetyLevel::Inert));
    }

    proptest! {
        #[test]
        fn any_runtime_option_token_is_denied(
            before in "[a-zA-Z.]{0,6}", after in "[a-zA-Z0-9.-]{0,8}",
            marker in prop::sample::select(vec!["+RTS", "-RTS", "--RTS", "+RTSx"]),
        ) {
            let cmd = format!("shellcheck {before} {marker} {after}");
            prop_assert!(!crate::is_safe_command(&cmd), "{}", cmd);
        }
    }
}
