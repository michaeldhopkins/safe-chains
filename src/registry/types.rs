use serde::Deserialize;

use crate::verdict::SafetyLevel;

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

/// The differing knobs of a transfer command (`cp`/`mv`/`ln`): every source operand is read at
/// its own locus and the destination is a create/overwrite at its locus, but the source
/// *operation* and the clobber/recursion flags differ per command.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TomlTransfer {
    /// The source-operand operation: `observe` (cp/ln read the source into the dest/link) or
    /// `relocate` (mv removes the source from its old location).
    pub source: String,
    /// Flags whose PRESENCE means the destination will not be overwritten (`cp`/`mv`: `-n`,
    /// `--no-clobber`). Mutually exclusive with `clobber_flags`.
    #[serde(default)]
    pub no_clobber_flags: Vec<String>,
    /// Flags whose PRESENCE means the destination WILL be overwritten, the default being
    /// no-clobber (`ln`: `-f`, `--force`). Mutually exclusive with `no_clobber_flags`.
    #[serde(default)]
    pub clobber_flags: Vec<String>,
    /// Flags whose presence widens the scale to unbounded (`cp`: `-r`/`-R`/`-a`).
    #[serde(default)]
    pub recursive_flags: Vec<String>,
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
    /// `"file"` gates the first positional as an EXECUTOR through the execution-origin
    /// engine (worktree-local code allows, foreign denies) rather than the flat `level`.
    /// For interpreters run as `python3 ./s.py` / `ruby s.rb`. (`"project"` exists for subs
    /// but is not used on fallbacks.)
    #[serde(default)]
    pub executor: Option<String>,
    #[serde(default)]
    pub executor_redirect_flag: Option<String>,
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

/// One `[[command.sub.flag]]`: a flag that escalates its sub's classification when present.
#[derive(Debug, Deserialize)]
pub(super) struct TomlSubFlag {
    pub name: String,
    /// The archetype (`archetypes.toml`) this flag's presence ADDS to the profile — or
    /// `"unclassified"` to worst-case (fail-closed) a flag whose effect we can't yet name.
    pub classifies: String,
    /// Optional value-match: escalate only when the flag's VALUE starts with this prefix (space
    /// form `-c core.sshCommand=…` or glued `--flag=core.sshCommand=…`). Absent = escalate on the
    /// flag's mere PRESENCE (a bare flag like `--force`). This is what lets ONE valued flag be
    /// benign for most values and dangerous for a specific key (`git -c core.sshCommand=` = exec).
    #[serde(default)]
    pub value_prefix: Option<String>,
    /// `true` INVERTS the trigger: escalate when the flag is ABSENT, not present. For a SAFETY flag
    /// whose absence is the risk — `npm ci` runs lifecycle scripts UNLESS `--ignore-scripts` is
    /// given, so its base profile (local-install-pinned) escalates to supply-chain-build when
    /// `--ignore-scripts` is missing. Mutually exclusive with `value_prefix`.
    #[serde(default)]
    pub when_absent: Option<bool>,
    #[serde(default)]
    pub fact: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub judgment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TomlSub {
    pub name: String,
    #[serde(default)]
    pub candidate: Option<bool>,
    /// A facet archetype name (`archetypes.toml`) — the Phase-1 successor to `candidate = true`:
    /// instead of hand-marking the sub above the line, it declares which recurring capability
    /// profile it is, and the engine DERIVES the verdict by projecting that profile through the
    /// levels. See `docs/design/behavioral-taxonomy-archetypes.md`.
    #[serde(default)]
    pub profile: Option<String>,
    /// Per-item research provenance for the classification (required when `profile` is set — the
    /// `every_profiled_sub_has_provenance` guard). Three layers so a future researcher can act on
    /// each precisely: `fact` = what the upstream tool DOCUMENTS (re-check `source` if it moves),
    /// the `profile` itself = our inference (which archetype it maps to), `judgment` = our stance
    /// where the source doesn't decide it (a policy call they may revisit). `source` cites the
    /// upstream doc/section. See `docs/design/behavioral-taxonomy-archetypes.md` §3.
    #[serde(default)]
    pub fact: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub judgment: Option<String>,
    /// Flags that NEUTRALIZE this sub's engine classification — its base `profile` AND its escalating
    /// `flag`s: when any is present, none applies and the sub falls through to its ordinary
    /// (`level`/`allow_all`) classification. For a sub dangerous BY DEFAULT (or in one flag mode) but
    /// safe when the output is diverted — `openssl rsa -in priv.pem` dumps the private key to stdout (a
    /// disclosure), but `-pubout`/`-out`/`-noout` divert it; `openssl pkcs12 -nodes` dumps the key, but
    /// `-out FILE` sends it to disk. Requires a `profile` or a `flag`; the sub keeps its `level`/
    /// `allow_all` for the neutralized case (a plain profiled sub force-denies its legacy kind, but an
    /// `unless_flags` sub needs the real classification for when a neutralizer is present).
    #[serde(default)]
    pub unless_flags: Vec<String>,
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
    /// Per-FLAG escalation + provenance (`[[command.sub.flag]]`): a flag that, when present, ADDS a
    /// capability to this sub's resolved profile — `git push --force` (→ destroy), `-c
    /// core.sshCommand=` (→ execution). The level algebra takes the max over the added capabilities,
    /// so a benign base + a dangerous flag lands at the flag's tier. See
    /// `docs/design/behavioral-taxonomy-archetypes.md` §3 (per-flag layer).
    #[serde(default)]
    pub flag: Vec<TomlSubFlag>,
    /// `true` marks the sub's first positional as a NETWORK DESTINATION whose *provenance* the
    /// engine classifies onto `locus.provenance` (established remote-name / literal URL / opaque
    /// `$VAR`), and whose command-transport form (`ext::<cmd>`) worst-cases as RCE. For
    /// `git push` and its kin (`scp`/`rsync`/`curl -d`). See `behavioral-taxonomy-exposure.md` §4.
    #[serde(default)]
    pub network_destination: Option<bool>,
    /// A flag that ALSO carries the destination and OVERRIDES the positional (`git push
    /// --repo=<dest>`). Classified with the same provenance rules — so `--repo=ext::sh` is caught as
    /// RCE. Requires `network_destination`.
    #[serde(default)]
    pub destination_flag: Option<String>,
    /// Flags whose VALUE is a local output-file path, for a `data-export` sub (`supabase db dump
    /// -f`, `pg_dump --file`). When one is present the engine adds a path-gated write capability at
    /// that file's locus — a dump to `./out.sql` is a worktree write, one to `/etc/cron.d/job` a
    /// system write. Absent (the export goes to stdout) → no write, just the bulk remote read.
    /// Requires `profile` (only a `data-export` sub has an output file). See
    /// `behavioral-taxonomy-exposure.md`.
    #[serde(default)]
    pub output_path_flags: Vec<String>,
    #[serde(default)]
    pub nested_bare: Option<bool>,
    #[serde(default)]
    pub require_any: Vec<String>,
    #[serde(default)]
    pub first_arg: Vec<String>,
    /// First-positional globs (`secret`, `secret/*`) that make this sub a CREDENTIAL-READ: matching
    /// denies, before the allow-glob. The value-dependent complement to `profile=credential-read`
    /// (whole sub) — `kubectl get secret/x`, `aws configure get aws_secret_access_key`.
    #[serde(default)]
    pub credential_first_arg: Vec<String>,
    #[serde(default)]
    pub write_flags: Vec<String>,
    #[serde(default)]
    pub delegate_after: Option<String>,
    #[serde(default)]
    pub delegate_skip: Option<usize>,
    /// `"file"` (first positional is the executor path — `go run ./cmd`) or `"project"`
    /// (the current project is the executor — `cargo run`). Gates via the execution-origin
    /// engine instead of a flat level. See `DispatchKind::Executor`.
    #[serde(default)]
    pub executor: Option<String>,
    /// A valued flag whose value redirects the executor out of the project
    /// (`cargo run --manifest-path DIR/Cargo.toml`); its value is locus-gated. `Project` only.
    #[serde(default)]
    pub executor_redirect_flag: Option<String>,
    /// Predicate the executor path must satisfy (`"go-package"`), else deny. `File` only.
    #[serde(default)]
    pub positional_shape: Option<String>,
    #[serde(default)]
    pub handler: Option<String>,
    #[serde(default)]
    pub doc_body: Option<String>,
    /// Marks this sub's leaf invocation as safe inside
    /// `eval "$(CMD SUB ...)"`. The leaf is the deepest matched dispatch
    /// node — if this sub has nested sub-subs and the invocation matches
    /// deeper, the tag does NOT apply; the sub-sub must be tagged itself.
    /// Unset = not eval-safe (the default).
    #[serde(default)]
    pub eval_safe: Option<bool>,
    /// Flag allowlist that extends `eval_safe = true` — these `-`-prefixed
    /// tokens are also permitted inside the substitution. Default empty,
    /// meaning only the bare form plus positionals are eval-safe.
    /// Build panics if this is set without `eval_safe = true`.
    #[serde(default)]
    pub eval_safe_flags: Vec<String>,
    /// Per-valued-flag value allowlist (same semantics as the
    /// command-level field). Maps each valued flag (which MUST also
    /// appear in `eval_safe_flags`) to its permitted values.
    #[serde(default)]
    pub eval_safe_flag_values: std::collections::HashMap<String, Vec<String>>,
    /// Flags where AT LEAST ONE must appear (same semantics as the
    /// command-level field).
    #[serde(default)]
    pub eval_safe_required_flags: Vec<String>,
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
    /// Top-level classifying flags (`[[command.flag]]`), lowered from `TomlSubFlag`. A present flag
    /// classifies the whole invocation as its archetype — read by `registry::command_flag_archetypes`
    /// → `engine::resolve::resolve` (the flat-command analog of a profiled sub's escalating flags).
    pub(super) archetype_flags: Vec<FlagProvenance>,
    /// Declarative facet behavior (`[command.behavior]`), lowered to typed facet enums. Read
    /// by `registry::command_behavior` → `engine::resolve::resolve_behavior`. When present,
    /// the engine classifies this command from its declared facets instead of a Rust resolver.
    pub(super) behavior: Option<BehaviorSpec>,
    pub(super) kind: DispatchKind,
}

/// A command's declarative facet behavior, lowered from `[command.behavior]` (`TomlBehavior`)
/// with every facet string resolved to its enum at build time. The generic resolver reads this
/// plus the tokens and builds a `Profile`. Clone so it can be attached uniformly across the
/// `build_command` construction sites.
#[derive(Debug, Clone)]
pub(crate) struct BehaviorSpec {
    pub operation: crate::engine::facet::Operation,
    pub positionals: PositionalRole,
    pub scale: ScaleModel,
    /// Behavior's own flag grammar, pre-split for the shared `walk_positionals`.
    pub short: Vec<u8>,
    pub valued_short: Vec<u8>,
    pub long: Vec<String>,
    pub valued_long: Vec<String>,
    pub numeric_shorthand: bool,
    /// Flags whose presence widens the scale to unbounded (`rm -r`, `grep -r`).
    pub unbounded_flags: Vec<String>,
    /// Valued flags whose VALUE is a path to gate (`touch -r REF` reads REF), with its role.
    pub path_flags: Vec<PathFlag>,
    pub hook: Option<BehaviorHook>,
    /// Transfer semantics, present iff `positionals == Transfer`.
    pub transfer: Option<TransferSpec>,
}

/// A valued flag whose value is a path safe-chains gates by locus. One spelling per entry (a
/// flag with both short and long forms is two entries); the resolver scans for each.
#[derive(Debug, Clone)]
pub(crate) struct PathFlag {
    pub short: Option<u8>,
    pub long: Option<String>,
    pub role: PathRole,
}

/// The role a path-flag's value plays — read (gated by read locus) or write (by write locus).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathRole {
    Read,
    Write,
}

/// Lowered `[command.behavior.transfer]` — the per-command transfer knobs, terms resolved.
#[derive(Debug, Clone)]
pub(crate) struct TransferSpec {
    pub source: TransferSource,
    pub no_clobber_flags: Vec<String>,
    pub clobber_flags: Vec<String>,
    pub recursive_flags: Vec<String>,
}

/// The source-operand operation of a transfer command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferSource {
    /// cp/ln: read the source into the destination/link (no disclosure to the model).
    Observe,
    /// mv: remove the source from its old location (trivially reversible).
    Relocate,
}

/// The closed set of operand-role shapes (§ design doc: the `Operands` enum, as data).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PositionalRole {
    None,
    Read,
    Write,
    PatternThenRead,
    Transfer,
}

/// How a command's `Scale` is computed from its operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScaleModel {
    /// Every operation is a single item regardless of operand count (cat/head).
    Single,
    /// Count, glob, or a recursion flag widen it (`breadth_scale`) — rm/mkdir.
    Breadth,
}

/// A named thin resolver hook for irreducible token logic a declaration can't express — a
/// command whose operand syntax is not getopt positional (grep's pattern disambiguation, dd's
/// `key=value`, tar's dashless mode bundles, sed's mini-language script). The hook parses the
/// tokens; the facets still come from the declaration + the builders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BehaviorHook {
    Grep,
    Dd,
    Tar,
    Sed,
}

/// Runtime form of a `[[command.sub.flag]]` — the engine-relevant part of an escalating flag: its
/// `name` (matched against the tokens) and the archetype it `classifies` as when present. Its
/// research provenance (`fact`/`source`/`judgment`) lives on the TOML side and is validated at build
/// time, not carried here.
#[derive(Debug, Clone)]
pub(super) struct FlagProvenance {
    pub name: String,
    pub classifies: String,
    /// See `TomlSubFlag::value_prefix` — `None` = escalate on presence; `Some` = only when the
    /// flag's value starts with this.
    pub value_prefix: Option<String>,
    /// See `TomlSubFlag::when_absent` — escalate when the flag is ABSENT (a safety flag whose
    /// absence is the risk).
    pub when_absent: bool,
}

#[derive(Debug, Clone)]
pub(super) struct SubSpec {
    pub name: String,
    pub kind: DispatchKind,
    /// The facet archetype this sub is classified as (`archetypes.toml`), if declared via
    /// `profile = …`. The engine resolves the sub to this archetype's static capability profile
    /// (`registry::sub_archetype`), deriving the verdict rather than taking a hand-marked level.
    /// (Its research provenance — `fact`/`source`/`judgment` — lives on the TOML side only and is
    /// validated at build time; it is not carried on the runtime spec.)
    pub profile: Option<String>,
    /// Escalating flags: each, when present, adds `classifies`'s capability to the resolved profile.
    pub flags: Vec<FlagProvenance>,
    /// Flags that NEUTRALIZE this sub's engine classification (`unless_flags`): when any is present,
    /// `sub_archetypes` drops BOTH the base `profile` and the escalating `flags`, so the sub falls
    /// through to its legacy classification (`openssl rsa -pubout`, `openssl pkcs12 -nodes -out`).
    pub unless_flags: Vec<String>,
    /// If this sub was declared with `policy = "key"`, the referenced
    /// handler_policy name is preserved for docs rendering so a sub
    /// that points at a policy also shown in **Shared flag sets** can
    /// render as a reference rather than duplicating the flag list.
    pub policy_ref: Option<String>,
    /// True when this sub's leaf invocation is tagged as safe-to-eval.
    /// Walked by `registry::is_eval_safe_invocation()`.
    pub eval_safe: bool,
    /// Flag allowlist extending `eval_safe` — flags permitted in the
    /// substituted invocation when the walker stops at this sub.
    pub eval_safe_flags: Vec<String>,
    /// Per-valued-flag value allowlist (same semantics as on
    /// `CommandSpec`).
    pub eval_safe_flag_values: std::collections::HashMap<String, Vec<String>>,
    /// Flags where at least one must appear in the substituted
    /// invocation (same semantics as on `CommandSpec`).
    pub eval_safe_required_flags: Vec<String>,
    /// `true` = classify this sub's first positional as a network destination onto
    /// `locus.provenance` (see `TomlSub::network_destination`).
    pub network_destination: bool,
    /// A flag that overrides the positional destination (`git push --repo=…`); see
    /// `TomlSub::destination_flag`.
    pub destination_flag: Option<String>,
    /// Output-file flags for a `data-export` sub; a present one adds a path-gated write capability
    /// at the file's locus (see `TomlSub::output_path_flags`).
    pub output_path_flags: Vec<String>,
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
        /// First-positional globs that classify the invocation as a credential-read (deny), checked
        /// after explicit subs and before the allow-glob. Empty for almost every command.
        credential_first_arg: Vec<String>,
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

impl ExecutorKind {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        match name {
            "file" => Some(Self::File),
            "project" => Some(Self::Project),
            _ => None,
        }
    }
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
    /// See `DispatchKind::Executor::redirect_flag`. Unused for `File` fallbacks.
    pub executor_redirect_flag: Option<String>,
}
