//! The two command-wide switches that act around dispatch rather than inside it, lowered from
//! `env_assignment_positionals` and `ghc_rts` in a command TOML.

use crate::parse::Token;
use crate::verdict::Verdict;

const RUNTIME_OPTIONS_LINE: &str =
    "Any `+RTS`, `-RTS` or `--RTS` argument is refused (GHC runtime options can write statistics to a named file).";

/// `env_assignment_positionals`: the command PUTS its `NAME=VALUE` positionals into the environment
/// (`export`, `declare -x`), so each is classified through `envvars.toml` exactly as the
/// `VAR=value cmd` prefix form is, and the two spellings of one capability cannot disagree.
///
/// `ghc_rts`: the command is a program built with GHC, whose runtime reads `+RTS` options from its
/// arguments and lets them write statistics to a named file, so any argument equal to `+RTS`,
/// `-RTS` or `--RTS` denies the whole invocation, wherever it sits.
#[derive(Debug, Clone, Copy)]
pub(in crate::registry) struct SpecHooks {
    env_assignment_positionals: bool,
    ghc_rts: bool,
}

impl SpecHooks {
    pub(in crate::registry) fn lower(env_assignment_positionals: Option<bool>, ghc_rts: Option<bool>) -> Self {
        Self { env_assignment_positionals: env_assignment_positionals.unwrap_or(false), ghc_rts: ghc_rts.unwrap_or(false) }
    }

    pub(in crate::registry) fn refuses(&self, tokens: &[Token]) -> bool {
        self.ghc_rts && tokens.iter().any(|t| crate::ghc_rts::is_runtime_option(t.as_str()))
    }

    pub(in crate::registry) fn classify_exports(&self, tokens: &[Token], verdict: Verdict) -> Verdict {
        if !self.env_assignment_positionals {
            return verdict;
        }
        crate::envvars::exported_verdict(tokens[1..].iter().map(Token::as_str)).combine(verdict)
    }

    pub(in crate::registry) fn describe(&self, description: String) -> String {
        if !self.ghc_rts {
            description
        } else if description.is_empty() {
            RUNTIME_OPTIONS_LINE.to_string()
        } else if description.starts_with("- ") {
            format!("{description}\n- {RUNTIME_OPTIONS_LINE}")
        } else {
            format!("{description} {RUNTIME_OPTIONS_LINE}")
        }
    }
}
