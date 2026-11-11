//! The `[command.handler_policy]`, `[command.fallback]` and `[command.wrapper]` tables as written in
//! a command TOML, out of `types.rs`, and the whole-line checks (`LineChecks`) a command declares.

use serde::Deserialize;

use super::TomlLevel;
use crate::verdict::Verdict;

#[derive(Debug, Deserialize)]
pub(in crate::registry) struct TomlHandlerPolicy {
    #[serde(default)]
    pub standalone: Vec<String>,
    #[serde(default)]
    pub valued: Vec<String>,
    #[serde(default)]
    pub optional_valued: Vec<String>,
    #[serde(default)]
    pub bare: Option<bool>,
    #[serde(default)]
    pub max_positional: Option<usize>,
    #[serde(default)]
    pub tolerate_unknown_short: Option<bool>,
    #[serde(default)]
    pub tolerate_unknown_long: Option<bool>,
    #[serde(default)]
    pub numeric_dash: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(in crate::registry) struct TomlFallback {
    #[serde(default)]
    pub level: Option<TomlLevel>,
    #[serde(default)]
    pub bare: Option<bool>,
    #[serde(default)]
    pub max_positional: Option<usize>,
    #[serde(default)]
    pub standalone: Vec<String>,
    #[serde(default)]
    pub valued: Vec<String>,
    #[serde(default)]
    pub optional_valued: Vec<String>,
    #[serde(default)]
    pub tolerate_unknown_short: Option<bool>,
    #[serde(default)]
    pub tolerate_unknown_long: Option<bool>,
    #[serde(default)]
    pub numeric_dash: Option<bool>,
    /// Named predicate the handler applies to the first positional arg.
    /// Currently the only value is `"path"` — accepts a token shaped like
    /// a file path (contains `/`, `.`, or is `-` for stdin). Adding new
    /// shapes is a one-line `PositionalShape` enum addition plus a match
    /// arm in `policy::positional_matches_shape()`.
    #[serde(default)]
    pub positional_shape: Option<String>,
    /// `"file"` gates the first positional as an EXECUTOR through the execution-origin
    /// engine (worktree-local code allows, foreign denies) rather than the flat `level`.
    /// For interpreters run as `python3 ./s.py` / `ruby s.rb`. (`"project"` exists for subs
    /// but is not used on fallbacks.)
    #[serde(default)]
    pub executor: Option<String>,
    #[serde(default)]
    pub executor_redirect_flag: Option<String>,
    /// Tokens after the executor path are the SCRIPT's argv, not this command's arguments.
    /// Defaults to false, which is the enforcing answer — see `TomlSub::passes_argv`.
    #[serde(default)]
    pub passes_argv: Option<bool>,
    /// As on a command: any of these present raises the level to `SafeWrite`.
    #[serde(default)]
    pub write_flags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(in crate::registry) struct TomlWrapper {
    #[serde(default)]
    pub standalone: Vec<String>,
    #[serde(default)]
    pub valued: Vec<String>,
    #[serde(default)]
    pub positional_skip: Option<usize>,
    #[serde(default)]
    pub separator: Option<String>,
    #[serde(default)]
    pub bare_ok: Option<bool>,
    /// Accept one leading rustup toolchain selector (`cargo +nightly build`). A `+name` token is
    /// not a flag, so `standalone` cannot express it, and the name is variable, so it cannot be
    /// enumerated. Only meaningful on a structured (sub-dispatching) command.
    #[serde(default)]
    pub toolchain_selector: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(in crate::registry) struct TomlPassthrough {
    #[serde(default)]
    pub level: Option<TomlLevel>,
    #[serde(default)]
    pub bare: Option<bool>,
    #[serde(default)]
    pub max_positional: Option<usize>,
    #[serde(default)]
    pub guard: Option<String>,
    #[serde(default)]
    pub guard_short: Option<String>,
    #[serde(default)]
    pub require_any: Vec<String>,
    #[serde(default)]
    pub standalone: Vec<String>,
    #[serde(default)]
    pub valued: Vec<String>,
    #[serde(default)]
    pub optional_valued: Vec<String>,
    #[serde(default)]
    pub tolerate_unknown_short: Option<bool>,
    #[serde(default)]
    pub tolerate_unknown_long: Option<bool>,
    #[serde(default)]
    pub numeric_dash: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct PassthroughSpec {
    pub policy: super::OwnedPolicy,
    pub require_any: Vec<String>,
    pub level: crate::verdict::SafetyLevel,
}

#[derive(Debug, Clone)]
pub(in crate::registry) struct MatrixAction {
    pub policy_key: String,
    pub guard: Option<String>,
    pub guard_short: Option<String>,
}

/// What `dispatch_spec` checks across the whole line, beside the command's own grammar. The
/// command-wide switches that act around dispatch (`env_assignment_positionals`, `ghc_rts`) are
/// `SpecHooks`.
#[derive(Debug, Clone, Default)]
pub(in crate::registry) struct LineChecks {
    /// `flag_value_shapes = { "--toolchain" = "toolchain-name" }`: each named long flag's VALUE,
    /// as `--flag value` or `--flag=value`, must match its shape wherever it appears on the line.
    /// The whole line is scanned, inner commands included: a false refusal there only asks, where
    /// stopping early could miss the flag.
    pub flag_value_shapes: Vec<(String, crate::policy::PositionalShape)>,
}

impl LineChecks {
    pub(in crate::registry) fn lower(
        command: &str,
        flag_value_shapes: &std::collections::BTreeMap<String, String>,
    ) -> Result<Self, String> {
        let mut shapes = Vec::new();
        for (flag, name) in flag_value_shapes {
            let Some(shape) = crate::policy::PositionalShape::from_name(name) else {
                return Err(format!("command '{command}': unknown flag_value_shapes shape `{name}` for `{flag}`"));
            };
            if !flag.starts_with("--") {
                return Err(format!("command '{command}': flag_value_shapes names `{flag}`; only long flags are checked"));
            }
            shapes.push((flag.clone(), shape));
        }
        Ok(Self { flag_value_shapes: shapes })
    }

    pub(in crate::registry) fn judge(&self, tokens: &[crate::parse::Token], grammar: impl FnOnce() -> Verdict) -> Verdict {
        if !self.flag_values_match(tokens) {
            return Verdict::Denied;
        }
        grammar()
    }

    fn flag_values_match(&self, tokens: &[crate::parse::Token]) -> bool {
        self.flag_value_shapes.iter().all(|(flag, shape)| {
            tokens.iter().enumerate().skip(1).all(|(i, t)| {
                let t = t.as_str();
                if t == flag {
                    return tokens.get(i + 1).is_some_and(|v| shape.matches(v.as_str()));
                }
                t.strip_prefix(flag.as_str())
                    .and_then(|rest| rest.strip_prefix('='))
                    .is_none_or(|v| shape.matches(v))
            })
        })
    }
}
