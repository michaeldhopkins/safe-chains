use serde::Deserialize;

use crate::verdict::SafetyLevel;

#[derive(Debug, Deserialize)]
pub(super) struct TomlFile {
    pub command: Vec<TomlCommand>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TomlCommand {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub candidate: Option<bool>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub level: Option<TomlLevel>,
    #[serde(default)]
    pub bare: Option<bool>,
    #[serde(default)]
    pub max_positional: Option<usize>,
    /// Removed in favor of `tolerate_unknown_short` / `tolerate_unknown_long`.
    /// Build panics if any TOML still sets this — see SAMPLE.toml for the
    /// migration guidance. Kept on the deserializer struct so the panic
    /// message can name the offending command instead of a serde error.
    #[serde(default)]
    pub positional_style: Option<bool>,
    #[serde(default)]
    pub tolerate_unknown_short: Option<bool>,
    #[serde(default)]
    pub tolerate_unknown_long: Option<bool>,
    #[serde(default)]
    pub numeric_dash: Option<bool>,
    #[serde(default)]
    pub standalone: Vec<String>,
    #[serde(default)]
    pub valued: Vec<String>,
    #[serde(default)]
    pub bare_flags: Vec<String>,
    #[serde(default)]
    pub sub: Vec<TomlSub>,
    #[serde(default)]
    pub handler: Option<String>,
    #[serde(default)]
    pub doc_body: Option<String>,
    #[serde(default)]
    pub require_any: Vec<String>,
    #[serde(default)]
    pub first_arg: Vec<String>,
    #[serde(default)]
    pub wrapper: Option<TomlWrapper>,
    #[serde(default)]
    pub write_flags: Vec<String>,
    #[serde(default)]
    pub researched_version: Option<String>,
    /// Sample invocations that double as test fixtures.
    /// `examples_safe` must validate as Allowed; `examples_denied` must validate as Denied.
    /// Use these to exercise aliases and canonical forms (e.g. `mise use` and `mise u`)
    /// so drift between the TOML and runtime dispatch fails the test suite.
    #[serde(default)]
    pub examples_safe: Vec<String>,
    #[serde(default)]
    pub examples_denied: Vec<String>,
    /// Shortcut: every invocation of this command is denied. Used in custom
    /// TOMLs to lock down a built-in (e.g. `name = "gh", deny = true` in
    /// `.safe-chains.toml` denies every gh form for that project).
    #[serde(default)]
    pub deny: Option<bool>,
    /// Alternate grammar engaged when standard sub-dispatch finds no match.
    /// Only meaningful for handler-using commands (e.g. tilt's Ruby template
    /// engine fallback when no Kubernetes tilt sub matches). The handler is
    /// responsible for invoking it via `registry::try_fallback_grammar()`.
    #[serde(default)]
    pub fallback: Option<TomlFallback>,
    /// Named flag policies the handler references by string key. Used when
    /// a handler's dispatch logic genuinely can't move to TOML (e.g. gh's
    /// sub × action matrix) but the per-policy WordSets are still data that
    /// should live in TOML. The handler reads them via
    /// `registry::check_handler_policy(cmd, key, tokens)`.
    #[serde(default)]
    pub handler_policy: std::collections::HashMap<String, TomlHandlerPolicy>,
    /// Named string lists the handler references by string key. Same
    /// motivation as `handler_policy` but for plain word sets (allowed
    /// subcommand names, ini directives, etc.). Read via
    /// `registry::handler_word_list(cmd, key)`.
    #[serde(default)]
    pub handler_data: std::collections::HashMap<String, Vec<String>>,
    /// Parent × action → policy matrices. One block declares: "for
    /// these parent subcommand names, each of these action verbs maps
    /// to a named `handler_policy` and validates at this safety level."
    /// Lets handlers express their dispatch tables as data instead of
    /// `match` arms. Walked by `registry::try_matrix_dispatch()`.
    #[serde(default)]
    pub matrix: Vec<TomlMatrix>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TomlMatrix {
    pub parents: Vec<String>,
    pub level: TomlLevel,
    pub actions: std::collections::HashMap<String, TomlMatrixAction>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum TomlMatrixAction {
    /// Shorthand: `list = "policy_name"` — references handler_policy by
    /// name; no guard required.
    Policy(String),
    /// Detailed form: `download = { policy = "release_download", guard
    /// = "--output", guard_short = "-O" }`. The guard flag must be
    /// present in the action's args for the dispatch to succeed.
    Detailed {
        policy: String,
        #[serde(default)]
        guard: Option<String>,
        #[serde(default)]
        guard_short: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
pub(super) struct TomlHandlerPolicy {
    #[serde(default)]
    pub standalone: Vec<String>,
    #[serde(default)]
    pub valued: Vec<String>,
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
pub(super) struct TomlFallback {
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
}

#[derive(Debug, Deserialize)]
pub(super) struct TomlWrapper {
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
}

#[derive(Debug, Deserialize)]
pub(super) struct TomlSub {
    pub name: String,
    #[serde(default)]
    pub candidate: Option<bool>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub level: Option<TomlLevel>,
    #[serde(default)]
    pub bare: Option<bool>,
    #[serde(default)]
    pub max_positional: Option<usize>,
    /// Removed; see TomlCommand::positional_style.
    #[serde(default)]
    pub positional_style: Option<bool>,
    #[serde(default)]
    pub tolerate_unknown_short: Option<bool>,
    #[serde(default)]
    pub tolerate_unknown_long: Option<bool>,
    #[serde(default)]
    pub numeric_dash: Option<bool>,
    #[serde(default)]
    pub standalone: Vec<String>,
    #[serde(default)]
    pub valued: Vec<String>,
    #[serde(default)]
    pub guard: Option<String>,
    #[serde(default)]
    pub guard_short: Option<String>,
    #[serde(default)]
    pub allow_all: Option<bool>,
    /// Reference a `[command.handler_policy.KEY]` block by name, copying
    /// its standalone/valued/bare/etc. into this sub's effective policy.
    /// Lets a single-sub form (search, browse, gh status) re-use the
    /// same flag list a matrix entry would, without duplicating the
    /// WordSets. Mutually exclusive with inline standalone/valued.
    #[serde(default)]
    pub policy: Option<String>,
    #[serde(default)]
    pub sub: Vec<TomlSub>,
    #[serde(default)]
    pub nested_bare: Option<bool>,
    #[serde(default)]
    pub require_any: Vec<String>,
    #[serde(default)]
    pub first_arg: Vec<String>,
    #[serde(default)]
    pub write_flags: Vec<String>,
    #[serde(default)]
    pub delegate_after: Option<String>,
    #[serde(default)]
    pub delegate_skip: Option<usize>,
    #[serde(default)]
    pub handler: Option<String>,
    #[serde(default)]
    pub doc_body: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub(super) enum TomlLevel {
    Inert,
    SafeRead,
    SafeWrite,
}

impl From<TomlLevel> for SafetyLevel {
    fn from(l: TomlLevel) -> Self {
        match l {
            TomlLevel::Inert => SafetyLevel::Inert,
            TomlLevel::SafeRead => SafetyLevel::SafeRead,
            TomlLevel::SafeWrite => SafetyLevel::SafeWrite,
        }
    }
}

#[derive(Debug)]
pub struct CommandSpec {
    pub name: String,
    pub description: String,
    pub aliases: Vec<String>,
    pub url: String,
    pub category: String,
    /// Upstream version of the underlying tool that was researched
    /// when this spec was last updated. Free-form string — e.g.
    /// `"1.9.0"`, `"v5.10.3"`, `"2026-05-08 master"`,
    /// `"@northflank/cli 0.10.15"`. Internal-only: not rendered in
    /// docs or used at runtime. Surfaces in tests and as a tripwire
    /// when researching newer versions of the same tool.
    pub researched_version: Option<String>,
    /// Sample invocations that the registry test runs through `is_safe_command`.
    /// Each `examples_safe` entry must produce `Verdict::Allowed`.
    pub examples_safe: Vec<String>,
    /// Sample invocations that must be denied. Use these to lock in security
    /// boundaries (e.g. `srb tc --metrics-file=/etc/passwd` should always
    /// be denied; recording it here catches regressions).
    pub examples_denied: Vec<String>,
    pub(super) kind: DispatchKind,
}

#[derive(Debug, Clone)]
pub(super) struct SubSpec {
    pub name: String,
    pub kind: DispatchKind,
}

#[derive(Debug, Clone)]
pub(super) enum DispatchKind {
    Policy {
        policy: OwnedPolicy,
        level: SafetyLevel,
    },
    FirstArg {
        patterns: Vec<String>,
        level: SafetyLevel,
    },
    RequireAny {
        require_any: Vec<String>,
        policy: OwnedPolicy,
        level: SafetyLevel,
        accept_bare_help: bool,
    },
    Branching {
        subs: Vec<SubSpec>,
        bare_flags: Vec<String>,
        bare_ok: bool,
        pre_standalone: Vec<String>,
        pre_valued: Vec<String>,
        first_arg: Vec<String>,
        first_arg_level: SafetyLevel,
    },
    WriteFlagged {
        policy: OwnedPolicy,
        base_level: SafetyLevel,
        write_flags: Vec<String>,
    },
    DelegateAfterSeparator {
        separator: String,
    },
    DelegateSkip {
        skip: usize,
    },
    Wrapper {
        standalone: Vec<String>,
        valued: Vec<String>,
        positional_skip: usize,
        separator: Option<String>,
        bare_ok: bool,
    },
    Custom {
        #[allow(dead_code)]
        handler_name: String,
        doc_body: Option<String>,
        /// TOML-declared subs the handler may consult via
        /// `registry::try_sub_dispatch()`. Empty unless the handler
        /// uses the helper.
        subs: Vec<SubSpec>,
        /// TOML-declared alternate grammar the handler may consult
        /// via `registry::try_fallback_grammar()`. `None` unless the
        /// handler uses the helper.
        fallback: Option<FallbackSpec>,
        /// Named flag policies the handler consults via
        /// `registry::check_handler_policy()`. Empty unless the handler
        /// has dispatch logic that picks a policy by name at runtime.
        handler_policies: std::collections::HashMap<String, OwnedPolicy>,
        /// Named string lists the handler reads via
        /// `registry::handler_word_list()`.
        handler_data: std::collections::HashMap<String, Vec<String>>,
        /// Sub × action matrices the handler walks via
        /// `registry::try_matrix_dispatch()`.
        matrices: Vec<MatrixSpec>,
    },
}

#[derive(Debug, Clone)]
pub struct OwnedPolicy {
    pub standalone: Vec<String>,
    pub valued: Vec<String>,
    pub bare: bool,
    pub max_positional: Option<usize>,
    pub tolerance: crate::policy::FlagTolerance,
}

#[derive(Debug, Clone)]
pub(super) struct MatrixSpec {
    pub parents: Vec<String>,
    pub level: SafetyLevel,
    pub actions: std::collections::HashMap<String, MatrixAction>,
}

#[derive(Debug, Clone)]
pub(super) struct MatrixAction {
    pub policy_key: String,
    pub guard: Option<String>,
    pub guard_short: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct FallbackSpec {
    pub policy: OwnedPolicy,
    pub level: SafetyLevel,
    pub positional_shape: Option<crate::policy::PositionalShape>,
}
