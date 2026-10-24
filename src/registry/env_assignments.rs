//! What a command does with its `NAME=VALUE` positionals, and how they are judged: as the
//! `VAR=value cmd` prefix when the command exports them (`export`, `declare -x`), and stricter when
//! it stores them for later shells (`mise set`).

use serde::Deserialize;

use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

/// `env_assignment_positionals` as a command file writes it: `true` for a command that exports its
/// assignments, `"stored"` for one that stores them in a file later shells export.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum DeclaredEnvAssignments {
    Exports(bool),
    Mode(Mode),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Mode {
    Stored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EnvAssignments {
    None,
    Exported,
    Stored,
}

impl EnvAssignments {
    pub(crate) fn lower(declared: Option<DeclaredEnvAssignments>) -> EnvAssignments {
        match declared {
            None | Some(DeclaredEnvAssignments::Exports(false)) => EnvAssignments::None,
            Some(DeclaredEnvAssignments::Exports(true)) => EnvAssignments::Exported,
            Some(DeclaredEnvAssignments::Mode(Mode::Stored)) => EnvAssignments::Stored,
        }
    }

    pub(crate) fn verdict(self, operands: &[Token]) -> Verdict {
        let operands = operands.iter().map(Token::as_str);
        match self {
            EnvAssignments::None => Verdict::Allowed(SafetyLevel::Inert),
            EnvAssignments::Exported => crate::envvars::exported_verdict(operands),
            EnvAssignments::Stored => crate::envvars::stored_verdict(operands),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct Probe {
        env_assignment_positionals: Option<DeclaredEnvAssignments>,
    }

    fn lowered(src: &str) -> Option<EnvAssignments> {
        toml::from_str::<Probe>(src).ok().map(|p| EnvAssignments::lower(p.env_assignment_positionals))
    }

    #[test]
    fn each_spelling_lowers_and_any_other_is_refused() {
        assert_eq!(lowered(""), Some(EnvAssignments::None));
        assert_eq!(lowered("env_assignment_positionals = false"), Some(EnvAssignments::None));
        assert_eq!(lowered("env_assignment_positionals = true"), Some(EnvAssignments::Exported));
        assert_eq!(lowered("env_assignment_positionals = \"stored\""), Some(EnvAssignments::Stored));
        for other in ["\"Stored\"", "\"exported\"", "\"\"", "1"] {
            assert_eq!(lowered(&format!("env_assignment_positionals = {other}")), None, "{other}");
        }
    }

    proptest::proptest! {
        /// Storing an assignment is never approved where exporting the same one is refused.
        #[test]
        fn storing_is_never_looser_than_exporting(
            words in proptest::collection::vec(proptest::prop_oneof!["[A-Za-z_]{1,8}=[ -~]{0,8}", "(PATH|PS1|http_proxy|FOO|LD_PRELOAD)=[./a-z]{0,6}", "-[a-z]"], 0..4),
        ) {
            let tokens: Vec<Token> = words.iter().map(|w| Token::from_raw(w.clone())).collect();
            if !EnvAssignments::Exported.verdict(&tokens).is_allowed() {
                proptest::prop_assert!(!EnvAssignments::Stored.verdict(&tokens).is_allowed(), "{:?}", words);
            }
        }
    }
}
