//! The `[command.handler_policy]`, `[command.fallback]` and `[command.wrapper]` tables as written in
//! a command TOML, out of `types.rs`.

use serde::Deserialize;

use super::TomlLevel;

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
