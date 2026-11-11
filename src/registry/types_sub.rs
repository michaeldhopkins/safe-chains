//! The sub declaration as written in a command TOML and as lowered to the runtime spec, out of
//! `types.rs`.

use serde::Deserialize;

use super::*;

/// One `[[command.sub.flag]]`: a flag that escalates its sub's classification when present.
#[derive(Debug, Deserialize)]
pub(in crate::registry) struct TomlSubFlag {
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
pub(in crate::registry) struct TomlSub {
    pub name: String,
    /// This sub also matches with ONE arbitrary trailing `:segment`, which inherits this sub's
    /// classification unchanged.
    ///
    /// For Rails' per-database rake tasks. A Rails 8 app has four databases out of the box
    /// (solid_cache, solid_queue, solid_cable), and rake generates a variant of each schema task
    /// per database: `db:migrate:primary`, `db:create:cache`, `db:drop:queue`. Fourteen base tasks
    /// times four databases is 56 names, and enumerating them does not even work — the segment is
    /// a key out of the app's own `config/database.yml`, so another app has `db:migrate:analytics`.
    ///
    /// Sound because the variant is strictly NARROWER than the base: `db:migrate:primary` migrates
    /// one of the databases `db:migrate` migrates all of. Inheriting the base's classification is
    /// therefore never a widening — `db:drop:cache` lands wherever `db:drop` already sits.
    ///
    /// Set it only where that containment argument holds. It is not a general "ignore the tail":
    /// the suffix must be a single plain identifier, and a sub that means something DIFFERENT with
    /// a suffix must keep declaring it separately.
    #[serde(default)]
    pub per_database: bool,
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
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub level: Option<TomlLevel>,
    #[serde(default)]
    pub bare: Option<bool>,
    #[serde(default)]
    pub max_positional: Option<usize>,
    #[serde(default)]
    pub min_positional: Option<usize>,
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
    pub optional_valued: Vec<String>,
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
    /// What this sub writes in the folder it runs in without naming a path: `"output"` (with
    /// `output_dirs`), `"source"`, or `"none"`. See `registry::cwd_writes`.
    #[serde(default)]
    pub writes_cwd: Option<String>,
    #[serde(default)]
    pub output_dirs: Vec<String>,
    /// Flags naming a network endpoint that must be on THIS machine (`--endpoint-url
    /// http://localhost:8000`). Two effects, both keyed on `netloc::is_loopback`: the flag is
    /// admitted only with a loopback value, and a loopback value re-classifies the sub's
    /// non-destroy capabilities as local (`resolve`'s loopback modifier). Declare only where a
    /// local emulator is a researched workflow for that service.
    #[serde(default)]
    pub loopback_valued: Vec<String>,
    /// Whether a `loopback_valued` flag naming this machine re-classifies the sub: `resolve` clears
    /// the destination-determined facets (remote reach, net direction, payload, metered cost) and
    /// leaves everything describing the operation alone. Absent = the sub keeps its remote
    /// classification whatever the destination; mandatory for destroy archetypes, build-enforced.
    #[serde(default)]
    pub loopback_localizes: Option<bool>,
    #[serde(default)]
    pub nested_bare: Option<bool>,
    #[serde(default)]
    pub require_any: Vec<String>,
    #[serde(default)]
    pub first_arg: Vec<String>,
    /// `first_arg` glob flags; empty = own and wrapper lists (`no_new_unresearched_first_arg_family`).
    #[serde(default)]
    pub first_arg_standalone: Vec<String>,
    #[serde(default)]
    pub first_arg_valued: Vec<String>,
    /// Flags admitted only when their VALUE names this machine (`--endpoint-url
    /// http://localhost:8000`). Same arity as `first_arg_valued`; the value is classified by
    /// `netloc::is_loopback`, and anything not positively recognized as loopback denies.
    #[serde(default)]
    pub first_arg_loopback_valued: Vec<String>,
    /// First-positional globs (`secret`, `secret/*`) that make this sub a CREDENTIAL-READ: matching
    /// denies, before the allow-glob. The value-dependent complement to `profile=credential-read`
    /// (whole sub) — `kubectl get secret/x`, `aws configure get aws_secret_access_key`.
    #[serde(default)]
    pub credential_first_arg: Vec<String>,
    #[serde(default)]
    pub flag_values: std::collections::HashMap<String, Vec<String>>,
    #[serde(default)]
    pub ascii_letter_flags: Vec<String>,
    /// Globs (`*` matches any run, including none) that make this sub refuse an invocation: tested
    /// against every argument that is not a flag, against the name and the value either side of an
    /// `=`, and against the value of a `--flag=value`. An argument whose text the shell decides
    /// (`$VAR`, a substitution) is refused whatever the globs say. A `delegate_after` sub is tested
    /// only on the arguments before its separator. For a sub that writes what a
    /// later command runs (`mise set FOO='{{ exec(command="id") }}'`).
    #[serde(default)]
    pub refuse_operands: Vec<String>,
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
    /// Tokens after the executor path are the SCRIPT's argv, not this command's own arguments.
    ///
    /// An interpreter passes them through (`python3 ./task.py --flag arg`) and its flag grammar
    /// cannot describe them, so only the prefix up to the script is checked. A tool that merely
    /// TAKES a path does not: a second positional on `karma start` is a second config file it
    /// loads and executes, and it must be counted by `max_positional`.
    ///
    /// Defaults to false, the enforcing answer, so a new File executor is governed by its own
    /// declared grammar unless someone states otherwise.
    #[serde(default)]
    pub passes_argv: Option<bool>,
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
    /// `[command.sub.output]` — what THIS SUB's stdout can name (same shape and
    /// semantics as the command-level `[command.output]`).
    ///
    /// Sub-scoped because the claim rarely holds for a whole multi-command tool:
    /// `git diff --name-only` prints worktree paths, while `git log` prints prose
    /// and `git config --get` prints whatever was configured. A command-level
    /// claim would have to be voided by an `invalidated_by` list naming every
    /// other subcommand, which is a denylist and fails open on the next one git
    /// adds.
    #[serde(default)]
    pub output: Option<TomlOutput>,
    #[serde(default)]
    pub passthrough: Option<TomlPassthrough>,
}

/// Runtime form of a `[[command.sub.flag]]` — the engine-relevant part of an escalating flag: its
/// `name` (matched against the tokens) and the archetype it `classifies` as when present. Its
/// research provenance (`fact`/`source`/`judgment`) lives on the TOML side and is validated at build
/// time, not carried here.
#[derive(Debug, Clone)]
pub(in crate::registry) struct FlagProvenance {
    pub name: String,
    pub classifies: String,
    /// See `TomlSubFlag::value_prefix` — `None` = escalate on presence; `Some` = only when the
    /// flag's value starts with this.
    pub value_prefix: Option<String>,
    /// See `TomlSubFlag::when_absent` — escalate when the flag is ABSENT (a safety flag whose
    /// absence is the risk).
    pub when_absent: bool,
}

/// How a sub's declared name is matched. An enum rather than a bool because the two are genuinely
/// different matching MODES, and because a third boolean on `SubSpec` is the point at which the
/// struct stops being readable — `name_match: WithDatabaseSuffix` says what `per_database: true`
/// only implied.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::registry) enum NameMatch {
    #[default]
    Exact,
    /// Also matches `name:<dbname>` for one plain identifier — see `TomlSub::per_database`.
    WithDatabaseSuffix,
}

#[derive(Debug, Clone)]
pub(in crate::registry) struct SubSpec {
    pub name: String,
    pub kind: DispatchKind,
    /// How this sub's name is matched against an argument.
    pub name_match: NameMatch,
    /// The facet archetype this sub is classified as (`archetypes.toml`), if declared via
    /// `profile = …`. The engine resolves the sub to this archetype's static capability profile
    /// (`registry::sub_archetype`), deriving the verdict rather than taking a hand-marked level.
    /// (Its research provenance — `fact`/`source`/`judgment` — lives on the TOML side only and is
    /// validated at build time; it is not carried on the runtime spec.)
    pub profile: Option<String>,
    /// Escalating flags: each, when present, adds `classifies`'s capability to the resolved profile.
    pub flags: Vec<FlagProvenance>,
    /// The sub's DECLARED flag allowlist, preserved for a PROFILED sub. Its legacy `kind` is forced
    /// to deny-all at build time, but the engine classifies a profiled sub straight from its
    /// archetype and never reaches that kind — so without keeping the lists here the declaration
    /// would be discarded and ANY flag would ride along on the profile. That was a fail-OPEN:
    /// `git rebase --exec 'rm -rf /'` and `supabase db dump --frobnicate` both classified as their
    /// benign base profile. `sub_archetypes` validates presented flags against these.
    pub allowed_standalone: Vec<String>,
    pub allowed_valued: Vec<String>,
    /// The sub's declared tolerance for flags it does not enumerate — the existing, explicit way to
    /// say "this tool's flag surface is genuinely unbounded" (a cloud API's per-service options).
    /// Preserved alongside the allowlist so the profiled path honors it: a sub that declares it stays
    /// open BY DECLARATION (reviewable in the TOML) rather than by a silent engine default, and a sub
    /// that does not — `git rebase`, where a flag changes the operation — enforces.
    pub allowed_unknown: crate::policy::UnknownTolerance,
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
    /// Lowered `[command.sub.output]` — this sub's own stdout claim, consulted by
    /// `registry::sub_output_locus` before the command-level one.
    pub output: Option<OutputSpec>,
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
    pub writes_cwd: Option<crate::pathctx::anchor::Anchor>,
    /// Endpoint flags gated on naming this machine; see `TomlSub::loopback_valued`.
    pub loopback_valued: Vec<String>,
    /// What a loopback endpoint buys this sub; see `TomlSub::loopback_localizes`.
    pub loopback_effect: LoopbackEffect,
    pub flag_values: Vec<(String, crate::registry::glob_flags::FlagValues)>,
    /// Globs an invocation of this sub may not match; see `TomlSub::refuse_operands`.
    pub refuse_operands: Vec<String>,
}

/// What a recognized loopback destination buys a sub. Gating the FLAG and re-classifying the
/// OPERATION are separate powers: a read only needs the former (it already passes), while a write
/// needs the latter to stop looking like a call to a cloud service.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::registry) enum LoopbackEffect {
    /// The endpoint flag is admissible when local; the classification is untouched.
    #[default]
    AdmitOnly,
    /// Also clears the facets the destination determines (remote reach, net direction, payload,
    /// metered cost), leaving the ones describing the operation alone.
    Localizes,
}
