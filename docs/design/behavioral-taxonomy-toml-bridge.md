# Behavioral taxonomy — expressing facets in TOML (the bridge)

*Status: Phase 0 LANDED, 2026-07-15. Goal: let a command declare its BEHAVIOR (facets) in TOML,
so classification lives in config, not a coarse `level` tag or a bespoke Rust resolver.*

**Phase 0 shipped** (the operand-role subset of the schema): 11 coreutils — cat, rm, grep, head, tail,
wc, mkdir, touch, cp, mv, ln — are classified by `[command.behavior]` and their Rust resolvers deleted.
`RESOLVERS` is down to `echo` + the irreducible parsers dd/tar/sed. The `Flags` struct is gone
(replaced by the shared `walk_*` slice functions). Fail-closed guards (`behavior_command_roster_is_pinned`,
`every_behavior_command_gates_hot_operands`, `every_hookless_behavior_command_worst_cases_unknown_flags`)
enumerate the registry and are red→green-proven. A 3-agent adversarial review found no critical
fail-open; its guard-layer findings are fixed. The transfer sub-block (`[behavior.transfer]`) covers
cp/mv/ln. **Not yet built:** the orthogonal-facet fields (network/disclosure/execution/secret/locus/
reversibility) for Phase 1 subcommand tools, and presets. The design below is the full target; §9's
schema draft is close to what shipped, modulo those deferred fields.

## 1. The problem

Today a command is classified two ways, and neither is "facets in TOML":

- **Legacy `level`** (`level = "SafeRead"`) — coarse; ~1,240 commands (all of `commands/*`). It flattens
  the facet axes into one of three tiers. It's the thing we keep tripping over ("is it read-only?").
- **Rust resolver** — a hand-written `fn(&[Token]) -> Profile` in the `RESOLVERS` table
  (`src/engine/resolve.rs`); ~15 commands (cat, grep, rm, cp, mv, ln, tar, sed, dd, …). This is the
  *real* facet model, but it's CODE. Adding a facet-classified command means writing Rust.

So "stop using legacy terms" has no alternative to point at: the only non-legacy path is Rust. That is
a regression toward the old days of a custom function per command. **We want the non-legacy path to be
TOML.**

## 2. The model (what a TOML must produce)

A command resolves to a **Profile** = a set of **Capabilities**. Each Capability is a bundle of facet
values (`src/engine/facet.rs`): `operation`, `locus {local, remote, binding}`, `scale`, `authority`,
`isolation`, `reversibility`, `persistence`, `disclosure`, `secret`, `network`, `execution`, `cost`.
The **level algebra** (`levels/default.toml`) already projects a Profile → a level (`inert ⊂ read-local
⊂ write-local ⊂ developer`). **That half is already data.** The missing half is command → Profile.

Crucially, the Profile depends on the TOKENS (which paths, which flags). So a TOML can't declare a
static Profile — it declares a **template** that a single generic resolver instantiates against the
tokens. The good news: the Rust resolvers already show the template is small and regular.

## 3. The vocabulary (porting the Rust to TOML)

The Rust already reveals the alphabet:

**Operand roles** — the `Operands` enum is 5 shapes. Port to `positionals = …`:
| Rust `Operands` | TOML `positionals` | commands |
|---|---|---|
| `None` | `"none"` | echo |
| `Paths` (read) | `"read"` | cat, head, tail, wc |
| `Paths` (write) | `"write"` | rm, mkdir, touch |
| `PatternThenPaths` | `"pattern-then-read"` | grep |
| `Transfer` | `"transfer"` | cp, mv, ln |
| `Custom` | (escape hatch → Rust) | dd, tar, sed |

**Operation + capability template** — the ~8 capability builders are named facet bundles. Port to
`operation = …` with a couple of tuning facets:
| Rust builder | `operation` | defaults it sets |
|---|---|---|
| `reads_content`/`observes` | `observe` | reversibility=none, persistence=transient |
| `creates` | `create` | reversibility=trivial, persistence=data |
| `overwrites` | `create` | reversibility=recoverable |
| `mutates` | `mutate` | reversibility=recoverable, persistence=data |
| `destroys` | `destroy` | reversibility=effortful |
| `relocates` | `mutate` | reversibility=trivial (mv src) |
| `executes` | `execute` | trust from `execution` |

**Per-flag overrides** — flags that change the profile. `[behavior.flags]`:
- a flag whose VALUE is a path with a role → `"-o" = { value = "write" }` (this is exactly today's
  `[command.path_gate]`, folded in);
- a flag that bumps a facet → `"-r" = { scale = "unbounded" }`;
- a flag that flips the operation → `"-i" = { operation = "mutate", positionals = "write" }` (sed -i).

**Static facets** — declared once, defaulted sensibly by operation: `network`, `execution`,
`disclosure`, `secret`, `locus.remote`, `reversibility`.

## 4. Examples (to review together)

Path commands — the current Rust resolvers, as TOML:

```toml
# cat — resolve_cat, in config
[command.behavior]
operation = "observe"
positionals = "read"            # each positional → observe @ classify_locus, disclosure=local-process
```
```toml
# rm — resolve_rm
[command.behavior]
operation = "destroy"
positionals = "write"
reversibility = "effortful"
[command.behavior.flags]
"-r" = { scale = "unbounded" }
"-R" = { scale = "unbounded" }
"-f" = {}                        # known/no-op for classification
```
```toml
# grep — resolve_grep
[command.behavior]
operation = "observe"
positionals = "pattern-then-read"
disclosure = "local-process"
[command.behavior.flags]
"-r" = { scale = "unbounded", positionals = "read" }
"-f" = { value = "read" }        # -f FILE reads a pattern file
# -P/-R handling stays as flag entries (P benign, R dereference → deny)
```

Subcommand tool — the BIG payoff (terraform), where each sub is a mostly-static profile:

```toml
[[command.sub]]
name = "plan"
[command.sub.behavior]
operation = "observe"
network = "outbound"             # authenticated remote read → refreshes state
locus = { remote = "trusted-remote" }
# projects to read-local-with-network → auto-approve, LIKE gcloud describe

[[command.sub]]
name = "apply"
[command.sub.behavior]
operation = "configure"
locus = { remote = "arbitrary" }
network = "outbound"
reversibility = "irreversible"
# projects ABOVE the auto-approve band → DENIED *by its declared behavior*
```

Note what happened to `apply`: it is no longer `candidate = true` (a hand-marked exclusion). It is
denied *because its declared facets place it above the band*. **The classification is the reason.** This
is the "research confusion goes away" payoff — you describe what it does, and safety falls out.

## 5. The challenges (the hard part — to enumerate together)

1. **Token-dependence is a template, not a value.** Fine for the operand-role patterns; the generic
   resolver instantiates per token. The escape hatch (Rust) remains for irregular syntax.
2. **Flag-conditional facets.** `sed -i` (read→write), `tar` MODE CHARS (`c`/`x`/`t` aren't `-flags`),
   `grep -r` (→ unbounded). The `[behavior.flags]` map covers normal flags; tar's dashless mode
   bundles and sed's in-script `w`/`e`/`r` are genuinely irregular → likely stay Rust (their DATA is
   already TOML-able; their PARSER is logic).
3. **Delegation.** `find -exec`, `xargs`, `env`, `timeout`, `bundle exec`, `cargo run` — the profile IS
   the inner command's (or an executor-locus gate). Already handled by the wrapper/executor/delegate
   primitives; those stay as declarative-but-logic primitives (they already live partly in TOML).
4. **Embedded-code analysis.** `perl -e`, `awk`, `sed` scripts, `mlr` verbs — real parsers. Stay as
   handlers; their DATA already moved to TOML (the `verb-chain` primitive is the template).
5. **Multi-capability profiles.** `cp` = read(source) + write(dest). `positionals = "transfer"` yields
   both; a command needing several distinct caps may list a `[[behavior.capability]]` array.
6. **Defaults must fail CLOSED — this is the load-bearing safety property.** Safe-ward defaults are a
   fail-open bug: a command that under-declares (typo'd `rm`, half-finished new sub) would silently
   classify safe and auto-approve. So **an unstated orthogonal facet takes its WORST term**, exactly
   like the level algebra's unrecognized→worst rule. An under-specified command projects *high* →
   denied; you never accidentally auto-approve, you accidentally deny (annoying, safe) and fix it by
   declaring more. Benign is something a command must PROVE, three ways only:
   - **Entailment** (safe, derived from a stated fact): `operation="observe"` cannot be `destroy`/
     `irreversible` — a derivation, not an assumption.
   - **Explicit assertion**: the command names the benign value (`network = "none"`).
   - **Preset** (`profile = "pure-filter"`): a named, audited bundle expanding to the full 12-axis
     benign assertion for a shape (echo/tr/sort), defined ONCE where a reviewer sees all axes at once —
     so an inert command is one reviewed line, not twelve silent defaults. The verbosity is paid per
     *shape*, not per command, and never by omission.
7. **Coexistence during migration.** A command has `[behavior]` (new) OR `level` (legacy); the dispatch
   prefers `behavior`, falls back to `level`. Convert incrementally, engine stays authoritative.

## 6. Phased plan

- **Phase 0 — schema + generic resolver, proven against the Rust we already have.** *(chosen start
  point — lowest risk, doubles as the faithfulness test for the fail-closed rule.)* Steps:
  1. `[behavior]` schema on `TomlCommand`/`TomlSub` (`registry/types.rs`): `operation`, `positionals`,
     `[behavior.flags]`, the orthogonal facet fields, and `profile` (preset ref). Deserialize only —
     no dispatch yet.
  2. Preset catalog (a `presets/*.toml` or a table in `levels/`): the audited benign bundles
     (`pure-filter`, `path-reader`, `path-writer`, `transfer`). Reviewed as units.
  3. ONE generic resolver in the engine: `(behavior, tokens) -> Profile`, applying fail-closed
     defaults (unstated orthogonal facet → worst term) + entailments + preset expansion.
  4. Port the simple RESOLVERS (echo/cat/head/tail/wc/rm/mkdir/touch/grep/cp/mv/ln) to `[behavior]`,
     DELETE their Rust entries, require existing engine tests to stay green. dd/tar/sed keep Rust.
  5. A guard test: every command with `[behavior]` and no explicit assertion on an orthogonal axis
     projects to a level ≥ what the worst-term rule demands — proves fail-closed isn't bypassable.
  Reviewable checkpoint before porting all 12: lock the schema shape on `grep` alone (richest simple
  case — pattern-then-read + `-r` scale + `-f` valued-path + `-P`/`-R` flag handling) and confirm it
  reproduces `resolve_grep` exactly.
- **Phase 1 — subcommand tools (biggest, most tractable swath).** terraform, gcloud, aws, kubectl,
  docker, git, … Each sub declares a static facet profile. `apply`/`destroy`/`push` become
  denied-by-classification, not `candidate`. This is where the friction and the "read-only confusion"
  actually live.
- **Phase 2 — flag-conditional facets.** Generalize `write_flags`, sed `-i`, the executor fields into
  the unified `[behavior.flags]` map.
- **Phase 3 — keep handlers only for genuine parsers** (interpreters, delegation). Their data is TOML;
  their logic is a small, audited set of primitives.
- **Then the ratchet has teeth:** new commands declare `[behavior]`; `level` is grandfathered and only
  shrinks. That's the test the user asked for — meaningful only once Phase 0/1 give a real alternative.

## 7. Open questions for review

- Positional operand model — RESOLVED (2026-07-14): **closed named shapes**
  (`none`/`read`/`write`/`pattern-then-read`/`transfer`), extended with a new named variant the day a
  real command needs one. NOT a free per-slot "arg N has role R" spec. Rationale: the free model is a
  mini-language that invites fitting-over-recognition (same failure as presets), mismatches variadic
  positionals (needs first/rest/last anchors that just re-spell the named shapes), and blows up the
  resolver's test surface past what an `every_*` guard can hold. We DO adopt path_gate's structure —
  its per-flag role map is already folded in as `[behavior.flags].kind = "read"/"write"/"exec"`, and
  its `shape` path-predicate folds in as an operand shape check — but the vocabulary stays closed.
- Preset catalog: what's the minimal set of audited `profile = …` bundles that covers most commands
  (pure-filter, path-reader, path-writer, cloud-read, cloud-mutate, …)? Each is defined once and
  reviewed as a unit; a command either references one or asserts each axis explicitly. No third path.
- Exactly which facets are ENTAILED by `operation` (safe to derive) vs ORTHOGONAL (must be asserted or
  preset-carried, never defaulted)? First cut: reversibility/persistence are entailed; network,
  execution, secret, disclosure, locus.remote are orthogonal.

## 8. The composition model (TOML + thin hook)

The Rust/TOML split is NOT per-command all-or-nothing. A command declares as much as possible in
`[behavior]`; where a genuinely-irreducible step remains, it names a thin hook that composes WITH the
declaration rather than replacing it. The hook produces the minimal command-specific output and hands
the rest back to the generic, TOML-driven path.

Composition seam — the generic resolver runs this pipeline, hook slots in at ONE step:
1. Walk tokens against the TOML flag table (standalone / valued / valued-is-path / pattern kinds) →
   {recognized flags, unknown-flag?, positional list, path-valued-flag values}.
2. Identify operands + roles: apply the declared `positionals` shape, OR — if `behavior_hook` is set —
   call the hook, which returns the classified operand set (and may narrow the operation, e.g. sed
   `-i`). This is the ONLY place a hook contributes.
3. For each operand: classify locus, emit a Capability from the TOML-declared `operation` + facets.
4. Fold in the asserted orthogonal facets; unstated → worst term (fail-closed).
5. Project via the level algebra.

So the hook's contract is small and uniform: `fn(tokens, &Behavior) -> Operands` (plus optional
operation override). It NEVER builds a Profile, assigns facets, or projects a level — those stay in
the TOML + generic path. grep's hook is only the three heuristics from §5-checkpoint (pattern-vs-file
disambiguation, `-r`→cwd, unknown-`--token`-is-pattern); its operation, disclosure, network, and flag
set are all declarative. This is how "use a custom resolver for necessary cases, working in
conjunction with the TOML as much as possible" is realized: the custom Rust is the exception slice, not
the command.

Migration corollary: an existing `RESOLVERS` entry is retired by *splitting* it — the facet/flag bulk
moves to `[behavior]`, and only the irreducible remainder (if any) stays as a hook. dd/tar/sed likely
keep a hook for their irregular operand syntax; echo/cat/head/tail/wc/rm/mkdir/touch/cp/mv/ln should
need no hook at all.
- Where exactly is the Rust/TOML line? RESOLVED (2026-07-14): **hybrid composition, not per-command
  either/or.** TOML carries the maximum; a thin optional hook carries only the irreducible token logic
  and DEFERS facets back to the TOML. See §8.

## 9. Sample definitions

Concrete artifacts for review — the Rust deserializer types, the preset file, and four commands.
KEY FACT that keeps this thin: the facet enums in `engine::facet` already carry their kebab strings
and a `from_term(&str) -> Option<Self>` parser. So a TOML field's value IS a facet term
(`operation = "observe"`, `network = "outbound"`); the build calls `from_term`, and an unknown term is
a BUILD ERROR (not a silent default). Benign is never gotten by omission.

### 9a. Deserializer types (`registry/types.rs`)

```rust
/// Declarative facet behavior (`[command.behavior]` / `[command.sub.behavior]`) — the
/// non-legacy classification path. The generic resolver reads this + the tokens and builds
/// a `Profile`, retiring a hardcoded `RESOLVERS` entry. Field values are facet term strings
/// from `engine::facet`; the build maps them via `FacetTerm::from_term` and errors on an
/// unknown term. Any orthogonal facet neither asserted here, entailed by `operation`, nor
/// carried by `profile` resolves to its WORST term at resolve time (fail-closed).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TomlBehavior {
    /// The act each operand capability performs (`Operation` term). Required unless a
    /// `profile` supplies it.
    #[serde(default)]
    pub operation: Option<String>,
    /// How bare positionals are touched — the operand-role the `Operands` enum encodes today:
    /// "none" | "read" | "write" | "pattern-then-read" | "transfer".
    #[serde(default)]
    pub positionals: Option<String>,
    /// An audited benign facet bundle, expanded before the assertions below
    /// (`profile = "pure-filter"`). Defined once in `presets/*.toml`.
    #[serde(default)]
    pub profile: Option<String>,
    /// Orthogonal-facet assertions shared with each flag override (see `TomlFacets`).
    #[serde(flatten)]
    pub facets: TomlFacets,
    /// Per-flag behavior — arity plus the facet deltas a flag applies when present.
    #[serde(default)]
    pub flags: std::collections::HashMap<String, TomlBehaviorFlag>,
    /// Thin custom hook for irreducible token logic only (grep's pattern-vs-file
    /// disambiguation). Composes: returns the classified operand set (and may narrow
    /// `operation`); facets + level projection stay declarative. Absent = pure declarative.
    #[serde(default)]
    pub hook: Option<String>,
}

/// The orthogonal facet axes, each a `FacetTerm` string. Unstated ⇒ worst term at resolve.
/// Shared by the base behavior and every flag override so a flag speaks the same vocabulary.
#[derive(Debug, Default, Deserialize)]
pub(super) struct TomlFacets {
    #[serde(default)] pub network: Option<String>,       // NetDirection, e.g. "outbound"
    #[serde(default)] pub disclosure: Option<String>,    // DisclosureAudience, "local-process"
    #[serde(default)] pub execution: Option<String>,     // ExecutionTrust, "self"
    #[serde(default)] pub secret: Option<String>,        // SecretLevel
    #[serde(default)] pub reversibility: Option<String>, // Reversibility
    #[serde(default)] pub scale: Option<String>,         // Scale (base, before flag bumps)
    #[serde(default)] pub locus: Option<TomlLocus>,      // { remote, local }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TomlLocus {
    #[serde(default)] pub remote: Option<String>,  // RemoteReach, "arbitrary"
    #[serde(default)] pub local: Option<String>,   // LocalLocus ceiling, "worktree"
}

/// One flag's contribution (`[command.behavior.flags."-r"]`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TomlBehaviorFlag {
    /// Arity/role: "standalone" (no value) | "valued" (consumes next, value ignored) |
    /// "read"/"write"/"exec" (consumes next AND classifies its locus, folding path_gate in) |
    /// "pattern" (supplies the pattern, so positional 0 is no longer the pattern).
    #[serde(default)]
    pub kind: Option<String>,
    /// Operation flip when present (sed `-i`: "mutate").
    #[serde(default)]
    pub operation: Option<String>,
    /// Positional-role flip when present (grep `-r`: "read").
    #[serde(default)]
    pub positionals: Option<String>,
    /// Behavior when this flag leaves no positionals (grep `-r` → "cwd").
    #[serde(default)]
    pub empty_positionals: Option<String>,
    /// Facet deltas applied when present — same vocabulary as the base.
    #[serde(flatten)]
    pub facets: TomlFacets,
}
```

At build these lower into a typed `BehaviorSpec` on `CommandSpec`/`SubSpec` (every string → its facet
enum via `from_term`, unknown → build error), exactly as `positional_shape: String` lowers to
`policy::PositionalShape`. `deny_unknown_fields` catches a mistyped KEY; `from_term` catches a mistyped
VALUE; an omitted axis fails closed. Three independent nets, all fail-safe.

### 9b. Preset catalog (`presets/filters.toml`)

Presets are SUGAR, never a unit of analysis. Hard rules, so they don't relapse into coarse tiers:
- A preset expands to explicit facet assertions ONLY — it can assert nothing a hand-written
  `[behavior]` couldn't; it saves typing, not thinking.
- The rendered/audit view of a command shows the EXPANDED facets, not the preset name — review always
  reads the axes, so a preset can never hide analysis behind a label.
- You reach for a preset AFTER determining a command's facets, when they match a common bundle — not
  before, as a bucket to sort into. Fitting a command to a preset is the anti-pattern.

```toml
# Audited benign bundles. A command references one via `profile = "…"`; it expands to these
# assertions, which per-field overrides may then tighten (never loosen silently — a loosening
# override still has to name a term, visible in review).
[pure-filter]     # echo, tr, sort, head/tail on stdin — pure stdout transform
operation = "observe"
network = "none"; execution = "self"; secret = "none"; disclosure = "local-process"
reversibility = "none"; scale = "single"
locus = { remote = "none" }
```

### 9c. Four commands

```toml
# cat — no hook, no flags of consequence
[command.behavior]
operation = "observe"
positionals = "read"          # each positional → observe @ classify_locus
disclosure = "local-process"
network = "none"; execution = "self"; secret = "none"   # asserted benign (fail-closed)
```

```toml
# rm — destroy; -r widens scale
[command.behavior]
operation = "destroy"
positionals = "write"
reversibility = "effortful"
network = "none"; execution = "self"; secret = "none"
[command.behavior.flags]
"-r" = { scale = "unbounded" }
"-R" = { scale = "unbounded" }
"-f" = { kind = "standalone" }
```

```toml
# grep — declarative bulk + a thin `hook` for the three heuristics (§5, §8)
[command.behavior]
operation = "observe"
positionals = "pattern-then-read"
disclosure = "local-process"
network = "none"; execution = "self"; secret = "none"
hook = "grep"                 # pattern-vs-file disambiguation, -r→cwd, unknown---token-is-pattern
[command.behavior.flags]
"-r" = { positionals = "read", empty_positionals = "cwd", scale = "unbounded" }
"--recursive" = { positionals = "read", empty_positionals = "cwd", scale = "unbounded" }
"-f" = { kind = "read" }      # -f FILE reads a pattern file (locus-classified)
"--file" = { kind = "read" }
"-e" = { kind = "pattern" }   # supplies the pattern → positional 0 becomes a path
```

```toml
# terraform — per-sub facet profiles; no legacy `level`, no hand-marked `candidate`
[[command.sub]]
name = "plan"
[command.sub.behavior]
operation = "observe"
network = "outbound"                    # authenticated remote read (refresh)
locus = { remote = "trusted-remote" }
# → read-local-with-network band → auto-approves, like `gcloud … describe`

[[command.sub]]
name = "apply"
[command.sub.behavior]
operation = "configure"
locus = { remote = "arbitrary" }
network = "outbound"
reversibility = "irreversible"
# → projects ABOVE the band → DENIED by its declared facets, not by a candidate flag
```

Note `apply` declares only four axes; the rest fail closed to worst — which for a remote/irreversible
`configure` only reinforces the deny. Under-declaration can never make `apply` *safer*. That asymmetry
is the whole safety argument for the schema.
