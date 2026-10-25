pub(crate) use super::dispatch::delegate::DelegateAfter;
use serde::Deserialize;

use crate::verdict::SafetyLevel;

#[path = "types_behavior.rs"]
mod behavior;
#[path = "shapes.rs"]
mod shapes;
#[path = "types_sub.rs"]
mod sub;
#[path = "types_tables.rs"]
mod tables;
#[path = "transfer.rs"]
mod transfer;
pub(crate) use behavior::*;
pub(super) use shapes::{misshapen, sub_shape};
pub(super) use sub::*;
pub(super) use tables::*;
pub(super) use transfer::TomlTransfer;
pub(crate) use transfer::{TransferSource, TransferSpec};

#[derive(Debug, Deserialize)]
pub(super) struct TomlFile {
    // Defaulted so a user config that contains only `[[trusted]]` (the repo-pin
    // list, parsed separately) is valid. Unknown tables like `[[trusted]]` are
    // ignored here.
    #[serde(default)]
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
    /// This command PUTS ITS `NAME=VALUE` POSITIONALS INTO THE ENVIRONMENT (`export`, `declare -x`), each classified through
    /// `envvars.toml` as the `VAR=value cmd` prefix form is (`exported_assignments_verdict`).
    #[serde(default)]
    pub env_assignment_positionals: Option<bool>,
    /// Removed for `tolerate_unknown_short`/`_long`; build panics if a TOML sets it, naming the command (SAMPLE.toml migrates).
    #[serde(default)]
    pub positional_style: Option<bool>,
    #[serde(default)]
    pub tolerate_unknown_short: Option<bool>,
    #[serde(default)]
    pub tolerate_unknown_long: Option<bool>,
    #[serde(default)]
    pub numeric_dash: Option<bool>,
    #[serde(default)]
    pub single_dash_words: Option<bool>,
    #[serde(default)]
    pub reads_commands_from: Vec<String>,
    #[serde(default)]
    pub standalone: Vec<String>,
    #[serde(default)]
    pub valued: Vec<String>,
    /// Flags that take a value OPTIONALLY: `--long` and `--long=27` are both real spellings.
    /// See `build_policy` for why this compiles down to the other two lists rather than adding
    /// a state to the walk.
    #[serde(default)]
    pub optional_valued: Vec<String>,
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
    /// Flags a `first_arg` GLOB family accepts. The glob admits an invocation on its first
    /// positional alone, so without these it never examines the flags at all and
    /// `--endpoint-url http://evil.com` rides along on a read. Empty = family not yet researched
    /// (permissive, grandfathered — see `no_new_unresearched_first_arg_family`).
    #[serde(default)]
    pub first_arg_standalone: Vec<String>,
    #[serde(default)]
    pub first_arg_valued: Vec<String>,
    /// Flags admitted only when their VALUE names this machine (`--endpoint-url
    /// http://localhost:8000`). Same arity as `first_arg_valued`; the value is classified by
    /// `netloc::is_loopback`, and anything not positively recognized as loopback denies.
    #[serde(default)]
    pub first_arg_loopback_valued: Vec<String>,
    #[serde(default)]
    pub credential_first_arg: Vec<String>,
    /// Top-level classifying flags (`[[command.flag]]`): a flag whose PRESENCE classifies the WHOLE
    /// invocation as an archetype — the flat-command analog of `[[command.sub.flag]]`. For a bimodal
    /// tool where a mode flag flips the operation: `age -d` / `sops --decrypt` reveal plaintext to the
    /// model (`decrypt-read`), while the bare/encrypt form is an ordinary local write. Resolved by
    /// `engine::resolve` via `registry::command_flag_archetypes`; each flag's `classifies` must name a
    /// known archetype and carry `fact`/`source` provenance (the `assert_command_flag_provenance` guard).
    #[serde(default)]
    pub flag: Vec<TomlSubFlag>,
    #[serde(default)]
    pub flag_valued: Vec<String>,
    #[serde(default)]
    pub wrapper: Option<TomlWrapper>,
    #[serde(default)]
    pub write_flags: Vec<String>,
    /// Path-argument gate co-located with the command (`[command.path_gate]`): the read/write
    /// role of each path-bearing flag value and of bare positionals. Consulted by
    /// `pathgate::should_deny` so a `--output`/`-i` path can't ship ungated. Same shape as
    /// `pathgates.toml`'s `[roles.X]`.
    #[serde(default)]
    pub path_gate: Option<crate::pathgate::RoleSpec>,
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
    /// Marks this command's leaf invocation as safe inside
    /// `eval "$(CMD ...)"`. Set on flat commands whose stdout is documented
    /// shell-init code (e.g. `ssh-agent`). The leaf is the deepest matched
    /// dispatch node — tagging here does NOT propagate to subs; each sub
    /// must be tagged independently. Unset = not eval-safe (the default).
    #[serde(default)]
    pub eval_safe: Option<bool>,
    /// Flag allowlist that extends `eval_safe = true` — these `-`-prefixed
    /// tokens are also permitted inside the substitution. Default empty,
    /// meaning only the bare form plus positionals are eval-safe.
    /// Build panics if this is set without `eval_safe = true`.
    #[serde(default)]
    pub eval_safe_flags: Vec<String>,
    /// Per-valued-flag value allowlist. Maps each valued flag (which
    /// MUST also appear in `eval_safe_flags`) to the set of values
    /// permitted in eval substitutions. Use for tools where the flag's
    /// value determines stdout shape (`aws --format env` vs
    /// `--format json`). Default empty = no value restriction beyond
    /// the bare-literal alphabet.
    #[serde(default)]
    pub eval_safe_flag_values: std::collections::HashMap<String, Vec<String>>,
    /// Flags where AT LEAST ONE must appear in the eval substitution.
    /// Use for tools whose bare invocation isn't shell-init code:
    /// `fzf` is interactive without `--bash|--zsh|--fish|--nushell`.
    /// Every entry must also appear in `eval_safe_flags`. Default
    /// empty = no required flags (bare invocation is fine).
    #[serde(default)]
    pub eval_safe_required_flags: Vec<String>,
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
    /// Parent × action → policy matrices. One block declares: "for
    /// these parent subcommand names, each of these action verbs maps
    /// to a named `handler_policy` and validates at this safety level."
    /// Lets handlers express their dispatch tables as data instead of
    /// `match` arms. Walked by `registry::try_matrix_dispatch()`.
    #[serde(default)]
    pub matrix: Vec<TomlMatrix>,
    /// A `verb-chain` grammar (`mlr`): a strict main-flag region followed by a
    /// `then`-chain of allowlisted verbs. Fully declarative — no handler needed.
    #[serde(default)]
    pub verb_chain: Option<TomlVerbChain>,
    /// Declarative facet behavior (`[command.behavior]`) — the non-legacy classification
    /// path. When present, the engine resolves this command by building a `Profile` from the
    /// declared operation + operand-role + flags (see `engine::resolve::resolve_behavior`),
    /// retiring a hardcoded `RESOLVERS` entry. The legacy `level` remains only as the
    /// fallback the engine already overrides.
    #[serde(default)]
    pub behavior: Option<TomlBehavior>,
    /// What this command's STDOUT can name (`[command.output]`) — the axis that decides whether a
    /// `$(…)` around it yields a bounded path or an unknowable one. Absent (the default) means
    /// unpinnable: the substitution worst-cases exactly as it always has. See
    /// docs/design/behavioral-taxonomy-substitution-locus.md.
    #[serde(default)]
    pub output: Option<TomlOutput>,
    /// What this command writes in its folder without naming a path; as on a sub.
    #[serde(default)]
    pub writes_cwd: Option<String>,
    #[serde(default)]
    pub output_dirs: Vec<String>,
}

/// `[command.output]` — a researched claim about where this command's stdout can POINT. It is a
/// separate axis from every facet: how safe a command is to RUN says nothing about what its output
/// names (`echo` is inert and `$(echo /etc/shadow)` names a credential file), so this is declared
/// per command or not at all.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TomlOutput {
    /// `operands` — output names paths beneath the command's own path operands (`fd`, `git
    /// ls-files`); `cwd` — output names the working directory (`pwd`).
    pub locus_from: String,
    /// Flags under which the claim does NOT hold, because they change what stdout CONTAINS.
    /// `fd -x cat {}` prints file contents rather than paths, and `fd -l` prints `ls -l` rows —
    /// neither is a path any more, so the locus rule cannot describe them. Any of these present
    /// makes the substitution unpinnable again.
    #[serde(default)]
    pub invalidated_by: Vec<String>,
    /// Value-taking flags, so their VALUE is not mistaken for an operand (`head -n 5` takes no
    /// file). Carried here rather than read off the legacy top-level `valued` for the same reason
    /// `[command.behavior]` carries its own grammar: this claim is researched as a unit, and a
    /// silently-shared list would let an unrelated edit change what counts as a root.
    #[serde(default)]
    pub valued: Vec<String>,
    /// Flags of which at least ONE must be present, or the claim does not hold. The inverse of
    /// `invalidated_by`, and needed by any command whose default output is not paths at all:
    /// `git diff` prints a PATCH, and only `--name-only` turns it into a list of paths. Empty
    /// (the normal case) means the claim holds for the bare invocation.
    #[serde(default)]
    pub requires: Vec<String>,
}

/// A command's declarative facet behavior (`[command.behavior]`). Field values that name a
/// facet term are the kebab strings from `engine::facet` (`operation = "observe"`); the build
/// maps them via `FacetTerm::from_term` and PANICS (naming the command) on an unknown term, so
/// a typo can't silently mis-classify. The behavior carries its OWN flag grammar
/// (`standalone`/`valued`), independent of the legacy top-level `standalone` — `rm`'s legacy
/// flag set is restricted to `--help`/`--version` (so the legacy fallback can't fail-open on
/// `rm -rf`), while its behavior grammar is the full destructive set.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TomlBehavior {
    /// The act each operand capability performs — an `Operation` term
    /// (`observe`/`create`/`mutate`/`destroy`/…).
    pub operation: String,
    /// How bare positionals are touched: `none` | `read` | `write` | `pattern-then-read` |
    /// `transfer` (the closed set the `Operands` enum encodes).
    pub positionals: String,
    /// Scale model: `single` (every read is one item — cat/head) or `breadth`
    /// (count/glob/recursion widen it — rm/mkdir). Defaults to `single`.
    #[serde(default)]
    pub scale: Option<String>,
    /// Boolean flags this command accepts (behavior's own grammar). Single-dash single-char
    /// tokens (`-r`) cluster; `--long` tokens are matched whole.
    #[serde(default)]
    pub standalone: Vec<String>,
    /// Value-taking flags (consume the next token or a glued `=value`).
    #[serde(default)]
    pub valued: Vec<String>,
    /// Accept the obsolete `-NUM` count shorthand (`head -20`).
    #[serde(default)]
    pub numeric_shorthand: Option<bool>,
    /// Per-flag facet deltas — a flag whose presence widens scale (`"-r" = { scale =
    /// "unbounded" }`), consumes a path value, or supplies a pattern.
    #[serde(default)]
    pub flags: std::collections::HashMap<String, TomlBehaviorFlag>,
    /// Thin custom hook for the irreducible token logic a declaration can't express
    /// (`grep`'s pattern-vs-file disambiguation). Composes: it returns the classified operand
    /// set; the facets + level projection stay declarative. Absent = pure declarative.
    #[serde(default)]
    pub hook: Option<String>,
    /// Transfer semantics (`[command.behavior.transfer]`), REQUIRED when `positionals =
    /// "transfer"` — the source-operand operation and the clobber/recursion flag sets that a
    /// `cp`/`mv`/`ln`-shaped command differs on.
    #[serde(default)]
    pub transfer: Option<TomlTransfer>,
}

/// One flag's contribution to a `[command.behavior]` profile: a scale bump when present, and/or
/// a path role on the flag's VALUE (a valued flag whose value is a path safe-chains must gate,
/// e.g. `touch -r REF` reads REF's timestamp — folds the `[command.path_gate]` idea into behavior).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TomlBehaviorFlag {
    /// Scale bump when present (`"-r" = { scale = "unbounded" }`).
    #[serde(default)]
    pub scale: Option<String>,
    /// The flag's VALUE is a path with this role: `read` (gated by its read locus) or `write`
    /// (gated by its write locus). The flag must be a valued flag (in `valued`).
    #[serde(default)]
    pub kind: Option<String>,
}

/// A `verb-chain` command grammar: `CMD [main-flags…] verb [args…] then verb [args…] …`
/// (`mlr`). The main-flag region is a STRICT allowlist (an unlisted flag denies — so a
/// mutating flag like mlr's `-I`/`--in-place`, omitted, is caught by omission); the verb
/// region is a `then`-chain where every verb NAME must be on the `verbs` allowlist (verb
/// ARGS are open-ended and not inspected — a pure verb has no shell/file escape).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TomlVerbChain {
    #[serde(default)]
    pub level: Option<TomlLevel>,
    /// The chain separator keyword (mlr: `then`). Defaults to `then`.
    #[serde(default)]
    pub separator: Option<String>,
    /// Boolean main flags (no value). SAFETY: every value-TAKING main flag must go in
    /// `main_valued` instead, or the walk mistakes its value for the verb boundary and a
    /// later mutating flag slips past in verb-land.
    #[serde(default)]
    pub main_standalone: Vec<String>,
    /// Value-taking main flags (`--from FILE`, `--ifs ,`), each consuming the next token.
    #[serde(default)]
    pub main_valued: Vec<String>,
    /// Variadic main flags (mlr `--mfrom A B …`) that consume tokens until a `--` terminator.
    #[serde(default)]
    pub main_variadic: Vec<String>,
    /// The allowlist of verb names permitted in every `then`-segment.
    #[serde(default)]
    pub verbs: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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
    Detailed(TomlMatrixActionDetailed),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TomlMatrixActionDetailed {
    pub policy: String,
    #[serde(default)]
    pub guard: Option<String>,
    #[serde(default)]
    pub guard_short: Option<String>,
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
    /// True when this command's bare invocation (no sub) is tagged as
    /// safe-to-eval. Walked by `registry::is_eval_safe_invocation()`.
    pub eval_safe: bool,
    /// Flag allowlist extending `eval_safe` — flags permitted in the
    /// substituted invocation when the walker stops at this node.
    pub eval_safe_flags: Vec<String>,
    /// Per-valued-flag value allowlist. When the walker hits a flag
    /// listed here, the value following the flag (separated by `=` or
    /// space) must be in this list.
    pub eval_safe_flag_values: std::collections::HashMap<String, Vec<String>>,
    /// Flags where at least one must appear in the substituted
    /// invocation. Empty = no required-flag constraint.
    pub eval_safe_required_flags: Vec<String>,
    /// The command's own path-argument gate (`[command.path_gate]`), if declared. Read by
    /// `registry::command_path_gate` → `pathgate::should_deny`.
    pub(super) path_gate: Option<crate::pathgate::RoleSpec>,
    /// `[[command.flag]]`, read by `registry::command_flag_archetypes` → `engine::resolve::resolve`.
    pub(super) archetype_flags: super::build::ArchetypeFlags,
    /// Declarative facet behavior (`[command.behavior]`), lowered to typed facet enums. Read
    /// by `registry::command_behavior` → `engine::resolve::resolve_behavior`. When present,
    /// the engine classifies this command from its declared facets instead of a Rust resolver.
    pub(super) behavior: Option<BehaviorSpec>,
    /// Lowered `[command.output]` — read by `registry::command_output_locus` →
    /// `engine::resolve::substitution_locus`, which decides whether a `$(…)` around this command
    /// yields a bounded path instead of the unpinnable sentinel.
    pub(super) output: Option<OutputSpec>,
    pub(super) writes_cwd: Option<crate::pathctx::anchor::Anchor>,
    /// True when the command's `NAME=VALUE` positionals become environment variables (`export`,
    /// `declare -x`). `dispatch_spec` then classifies each through `envvars::assignment_verdict`
    /// and combines the result, so `export LD_PRELOAD=/tmp/evil.so` denies as
    /// `LD_PRELOAD=/tmp/evil.so ls` does.
    pub(super) env_assignment_positionals: bool,
    pub(super) kind: DispatchKind,
}

#[derive(Debug, Clone)]
pub(super) enum DispatchKind {
    Policy {
        policy: OwnedPolicy,
        level: SafetyLevel,
        /// A predicate the first positional must meet, from the sub's `positional_shape`.
        shape: Option<crate::policy::PositionalShape>,
    },
    FirstArg {
        patterns: Vec<String>,
        level: SafetyLevel,
        /// Flags the glob family accepts. Empty = not yet researched (grandfathered); see
        /// `glob_presents_unlisted_flag`.
        standalone: Vec<String>,
        valued: Vec<String>,
        /// Flags admitted only when their value is a loopback endpoint.
        loopback_valued: Vec<String>,
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
        /// Flags the `first_arg` glob family accepts, as in `DispatchKind::FirstArg`.
        first_arg_standalone: Vec<String>,
        first_arg_valued: Vec<String>,
        first_arg_loopback_valued: Vec<String>,
        /// First-positional globs that classify the invocation as a credential-read (deny), checked
        /// after explicit subs and before the allow-glob. Empty for almost every command.
        credential_first_arg: Vec<String>,
        /// Accept one leading rustup toolchain selector (`cargo +nightly build`), stripped before
        /// sub dispatch. From `[command.wrapper] toolchain_selector = true`.
        toolchain_selector: bool,
    },
    WriteFlagged {
        policy: OwnedPolicy,
        base_level: SafetyLevel,
        write_flags: Vec<String>,
    },
    DelegateAfterSeparator(DelegateAfter),
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
    /// A `verb-chain` grammar (`mlr`): a strict main-flag region + a `then`-chain of
    /// allowlisted verbs. See `dispatch::dispatch_verb_chain`.
    VerbChain(VerbChainSpec),
    /// A code-execution command whose verdict is the execution-origin gate (worktree code
    /// allows, foreign denies), not a flat level. See `dispatch::dispatch_executor` and
    /// docs/design/behavioral-taxonomy-execution-origin.md.
    Executor {
        policy: OwnedPolicy,
        /// Verdict for a flag-only invocation with no executor (`python3 --version`).
        level: SafetyLevel,
        kind: ExecutorKind,
        /// A valued flag whose value REDIRECTS the executor out of the project
        /// (`cargo run --manifest-path DIR/Cargo.toml`) — its value is locus-gated like a
        /// file executor. Only meaningful for `ExecutorKind::Project`.
        redirect_flag: Option<String>,
        /// A predicate the executor path must satisfy, else deny (`ExecutorKind::File`).
        /// `go run` uses `go-package` so a remote import path (`rsc.io/x@latest`) is not
        /// treated as a worktree executor.
        shape: Option<crate::policy::PositionalShape>,
        /// Whether the tokens AFTER the executor path are the SCRIPT's argv rather than this
        /// command's own arguments. `python3 ./task.py --flag arg` passes them; `karma start
        /// ./a.conf.js` does not, and a second path there is a second CONFIG it will load.
        ///
        /// Decides how much of the invocation the flag policy governs, so it defaults to FALSE —
        /// the enforcing answer. See `dispatch::dispatch_executor`.
        passes_argv: bool,
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
        /// Sub × action matrices the handler walks via
        /// `registry::try_matrix_dispatch()`.
        matrices: Vec<MatrixSpec>,
    },
}

/// How a code-execution command locates its executor. `File`: the first positional is the
/// executor path (`bash x.sh`, `python3 x.py`, `go run ./cmd`). `Project`: the current
/// project is the executor and there is no path operand (`cargo run`, `dotnet run`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExecutorKind {
    File,
    Project,
}

pub use super::policy::OwnedPolicy;

#[derive(Debug, Clone)]
pub(super) struct MatrixSpec {
    pub parents: Vec<String>,
    pub level: SafetyLevel,
    pub actions: std::collections::HashMap<String, MatrixAction>,
}

#[derive(Debug, Clone)]
pub(super) struct VerbChainSpec {
    pub level: SafetyLevel,
    pub separator: String,
    pub main_standalone: Vec<String>,
    pub main_valued: Vec<String>,
    pub main_variadic: Vec<String>,
    pub verbs: std::collections::HashSet<String>,
}

#[derive(Debug, Clone)]
pub(super) struct FallbackSpec {
    pub policy: OwnedPolicy,
    pub level: SafetyLevel,
    pub positional_shape: Option<crate::policy::PositionalShape>,
    /// When set, the first positional is an EXECUTOR (a script/package the command runs),
    /// gated by the execution-origin engine instead of the flat `level`. `ExecutorKind::File`
    /// is the only form used by fallbacks (interpreters). See `dispatch::dispatch_executor`.
    pub executor: Option<ExecutorKind>,
    /// See `DispatchKind::Executor::redirect_flag` (unused for `File` fallbacks).
    pub executor_redirect_flag: Option<String>,
    /// See `DispatchKind::Executor::passes_argv`, set by interpreters, whose trailing tokens are argv.
    pub passes_argv: bool,
    pub write_flags: Vec<String>,
}
