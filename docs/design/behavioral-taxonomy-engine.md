# The profile-resolution engine (Stage 4)

Status: design (2026-07-05). Stage 4 of the roadmap (v1.3 §6): run a level predicate
against a resolved behavior profile, behind a flag, alongside the legacy classifier.
Grounded in the current classifier; file:line anchors are approximate and for
orientation. Not yet implemented.

This changes **classification**, not **parsing** — the CST, its fold, and the
delegation re-entry points are reused verbatim; only the leaf verdict of a
*converted* command is computed a new way.

## 0. Safety by positive assertion — the fail-closed rule

A profile is a **conjunction of positive safety claims**, never a set of detected
hazards. This is the allowlist principle applied *per facet*: safety on an axis is
something the resolver must *earn* by understanding the command form, and any axis it
cannot affirmatively bound resolves to that facet's **worst term** — never to zero.

The distinction is the whole ballgame, and there is a one-line test for it:

> **A facet classifier is allowlist-valid iff its behavior on *unrecognized* input is
> the worst term.**

`classify_locus` passes — an unpinnable `$VAR`/`..` path resolves to `machine`, and only
*recognized* shapes earn a lower rung. A hypothetical `classify_secret(path)` that
returns `secret = none` for paths off a known-secret list **fails** — it certifies
everything unlisted as safe *by omission*, which is a denylist in a classifier's
clothes. The same trap exists on every axis (a "network-command detector" leaks the
novel network tool); the cure is uniform: **fail closed**.

Consequences that shape the resolver:

- **The `§2.8` "facets default to their zero term" is a data-model convenience the
  resolver must not lean on.** `network = none` has to be earned ("`echo` provably opens
  no socket"), not inherited from a struct default. The resolver's finalizer
  worst-cases every facet a command form did not positively claim — a converted command
  that forgets an axis fails *safe* (asks), never *open*.
- **There is no secret path-detector.** `secret` splits into (a) *credential-extraction
  intent*, a positive per-**command** claim (`security find-generic-password -w`,
  `gpg -d`, `ssh-add` → `reads`; `cat`/`echo` → `none` — `cat` extracts no credential,
  it is a byte-reader), and (b) *content exposure*, carried by `disclosure` (audience)
  + `locus`, both fail-closed. `cat ~/.ssh/id_rsa` is denied because it is a
  `user`-scope `content-to-model` read, **not** because `id_rsa` was recognized — so the
  unanticipated `cat ~/.config/newtool/token` is denied by the same bound, for free.
  The residual risk a level *does* accept (e.g. `cat ./.env` at a level that trusts
  worktree content-reads) is then a **named, located** property of that level, not a gap
  hidden in an incomplete list.
- **Converting a command means certifying all twelve axes**, positively — the research
  bar rises from "is it dangerous?" to "can I prove it is bounded on every axis?". That
  cost is the price of a trustworthy allowlist.
- **Enforceable, not aspirational:** every facet classifier must be
  *worst-case-dominant on unrecognized input* — a proptest (feed adversarial arguments,
  assert nothing resolves below worst-case without a positive match) is the gate each
  classifier passes. This strengthens §2.3's worst-case rule from *unresolved values* to
  *unaddressed facets*.

## 0.1 Proposed (not settled): the attested-typical basis

§0 is binary — a facet claim is either **structural** (proven from the command form →
auto-approvable) or **worst-case** (unresolved → ask). That over-denies a real third
case: a read safe in the normal case but not provably always. `ps aux` is the exemplar —
its cross-principal argv is secret-free ~99.9% of the time, but can carry a password
passed on another process's command line. (Note the risk there is *content*, not
"cross-principal" per se, and it is partly **structural**: argv-presence is
flag-determined, so plain `ps -o pid,comm` is `structural`-safe and only the
argv-requesting form is uncertain.)

**Proposal:** give each safety claim an explicit **basis**, and the residual it implies:

| basis | residual | example |
|---|---|---|
| `structural` | none — provable from the form | `echo "lit"` (no fs); `ps -o pid,comm` (no argv) |
| `attested` | small, **named** — researched typicality | `ps aux`: "typically secret-free; residual = secret-in-argv" |
| `worst-case` | unbounded — unresolved | a `$VAR` path; an unresearched command |

A **level declares a residual tolerance**: a strict level auto-approves only
`structural`; a pragmatic level accepts `attested` (opting into named residuals); nothing
auto-approves `worst-case`. The *same* `ps aux` is then auto at a residual-tolerant level
and ask at a strict one — for a stated, reviewable reason.

Two properties keep it allowlist-honest:
- The basis is always a **positive** claim — a structural proof, or a researched
  attestation with a *named* residual — and its absence falls to `worst-case`. This is
  **not** safe-by-omission; it is the opposite of the old "auto-run with a *may hold a
  secret* parenthetical," which hid the residual instead of naming it.
- **Attesting is separate from auto-approving.** The `basis`/`residual` is a *fact about
  the command* (researched, recorded in `because`/evidence), independent of whether any
  shipped level acts on it. We can attest "`ps aux` is typically safe" without a default
  level auto-approving on it.

**Boundary:** this is reputation-adjacent (`delegation` B.5 — the taxonomy classifies the
*mechanism*, not the identity), so it rides as an annotation **beside** the facets (a
per-claim `basis`), never smeared into a facet ordinal. The facets stay crisp; the
epistemics ride alongside.

**Status: proposed, not settled.** `echo` (the first resolver slice) is pure `structural`
and unaffected; `ps` would be the first client. Open questions: whether `basis` attaches
per-facet-claim or per-capability; how `residual` is represented (a free-text name vs a
small enum of residual kinds); and whether any *default* level accepts `attested`, or it
stays strictly opt-in.

## 0.2 Residuals of static analysis (named, not hidden)

The resolver is a **pure function of the command string** — it never touches the
filesystem (which would be TOCTOU-racy against a mutable tree anyway). Three limits
follow, and the honest posture is to name them:

- **Symlinks/hardlinks defeat `locus`.** `classify_locus` sees the literal path, so a
  worktree symlink `./link → /etc/shadow` classifies as `worktree` and is admitted at a
  worktree-trusting level, though it reads a machine-scope file. The `..` normalization
  escape *is* caught (→ worst-case); symbolic links cannot be, statically. A residual of
  path-based classification, accepted by levels that trust worktree content.
- **Command identity is name + path, never binary content (a deliberate scope boundary).**
  safe-chains classifies by a command's executable name and path; verifying that a binary
  is genuinely the tool it is named is *out of scope* — this is a string classifier, not
  an integrity checker (see `CLAUDE.md` "Scope / trust model"). A bare `cat` trusts
  `$PATH` resolution to the real coreutils; a `$PATH` shadow is invisible. The resolver
  *does* worst-case a resolvable name reached via a non-standard path (`./cat`,
  `/tmp/cat`, `~/bin/grep`) — closing the planted-binary case — but a `$PATH`-shadowed
  bare `cat` is inherently trusted. (The legacy classifier keys purely on basename and
  hardens neither.)
- **TOCTOU.** Any static verdict can be invalidated by a filesystem change between check
  and exec. safe-chains is an allowlist gate, not a sandbox; it bounds what it *certifies*,
  not what the OS *enforces*.

These are the boundary of what a string-only, side-effect-free resolver can promise;
where a level accepts one, it is a stated property of that level.

## 1. Where the engine attaches

Today a command line is folded to a `Verdict` bottom-up: `command_verdict` →
`script_verdict` → `pipeline_verdict` → `cmd_verdict` → `simple_verdict`
(`src/cst/check.rs`), combining per-segment verdicts with `Verdict::combine` —
max-level, deny-absorbing (`src/verdict.rs`). The single leaf act is
`simple_verdict`'s call to `handlers::dispatch(&tokens)` (`src/cst/check.rs`),
which walks the handler chain and ends in `registry::toml_dispatch` →
`dispatch_spec` → `dispatch_kind` (`src/registry/dispatch.rs`), yielding
`Allowed(level)` or `Denied`.

The engine replaces exactly that leaf call, and nothing above it. For a converted
command, `simple_verdict` resolves a **behavior profile** and evaluates the active
level's predicate, projecting the result back to a `Verdict` so the existing fold,
redirect handling, substitution recursion, and chain semantics are untouched.
Redirects are the one leaf act that is *not* a command lookup —
`redirect_verdict`/`is_safe_write_target` (`src/cst/check.rs`) — and they become an
intrinsic capability on the enclosing command's profile (§2.3), not a separate
combine.

Two facets of the model do not fit a per-`Cmd` fold and run as a second pass at
`Script` scope (§3.1): the **flow policy** (a `secret`→outbound edge crosses a pipe;
`CmdSub`/`ProcSub` and the `Pipeline.commands` vector are the edges) and
**session-derived worst-casing**. Because both can only *deny* an otherwise-admitted
line, they compose with the deny-absorbing fold as additional deny sources — they
never rescue a denied segment.

## 2. Profile resolution

A resolved command line becomes a **profile**: a set of `capability` records (v1.3
§2.8). Capabilities attach to grammar nodes, classified by their effect on
facet-space:

- **additive** — contributes a capability (a leaf act: a `Policy` sub, a redirect
  write). The profile is the union.
- **modifier** — transforms the facets of capabilities already in scope, applied in a
  pinned order (§2.4). Never adds an operation of its own except the injection-point
  class (§3.5 of v1.3), which is additive-by-modifier.
- **gating** — narrows what a nested resolution may contribute (an isolation frame
  clamps nested `locus` to `sandbox-scope`; a `--` separator bounds where delegation
  begins).

### 2.1 Mapping the DispatchKind tree onto nodes

The existing `DispatchKind` tree (`src/registry/types.rs`) is already the node
structure; conversion re-labels each arm as a profile contribution rather than a
level literal:

| DispatchKind | node role | profile contribution |
|---|---|---|
| `Policy{level}` | additive leaf | one capability; its facets are the converted form of what `level` summarized |
| `FirstArg`, `RequireAny` | additive leaf, guarded | same, admitted only when the guard/pattern matches |
| `Branching{subs}` | interior node | recurse into the matched `SubSpec`; each sub is its own subtree of nodes |
| `WriteFlagged{write_flags}` | leaf + modifier | base capability; presence of a write flag is the `--force`-class modifier that raises `reversibility`/`persistence` |
| `Wrapper`, `DelegateAfterSeparator`, `DelegateSkip` | delegation frame | a **frame** (§2.2) over the nested profile obtained by re-entering the resolver on `tokens[i..]` |
| `Custom{handler}` | opaque or sub-forest | handler-decided; if it consults `try_sub_dispatch`/`try_fallback_grammar` the subs/fallback are ordinary nodes, else the profile is opaque → worst-case |

The `SafetyLevel` a leaf carries today (`Policy.level`, `first_arg_level`, sub
`level`) is the pre-conversion summary; converting the command replaces each with the
explicit facet vector it stood for. This is the only data edit conversion requires
(§7).

### 2.2 Delegation re-enters the resolver

`Wrapper`/`Delegate*` already reconstruct the inner command line and call
`command_verdict` recursively (`src/registry/dispatch.rs`). Under the engine that call
returns a nested **profile**, and the frame transforms it before union (v1.3 §3.1):

- `transparent` (`bash -c`, `env`, `xargs`) — identity; `env` with `LD_PRELOAD`/`PATH`
  adds `reconfiguring`.
- `privilege` (`sudo`, `doas`) — nested `authority := elevated/root`.
- `remote` (`ssh h CMD`, `docker exec`) — nested `locus.remote`; intrinsic outbound.
- `isolation` (`docker run`, `bwrap`) — clamp nested `locus.local` to
  `sandbox-scope`, re-add breach loci.
- `payload` (`psql -c`, `kubectl apply -f`) — nested is a typed document, decomposed
  by the four gates and resolution ladder (§3.1).
- `task-runner` (`make`, `npm run`) — nested is `ambient-config`, opaque unless a
  trusted-config root.
- `interactive` (bare `ssh`, a REPL) — nested is future input, unavailable → opaque,
  unbounded.

Frames compose (`sudo ssh h CMD` = privilege ∘ remote); nesting is shallow so
resolution runs to completion, and an unparseable inner command is opaque →
worst-case. Cross-segment compounding (`a && b | c`) is **not** delegation — the
`Script`/`Pipeline` fold already handles it; fan-out (`xargs`, `find -exec`) is the
`scale` facet, not a frame.

### 2.3 Argument resolution and worst-case

Locus, disclosure audience, network destination, and scale are functions of argument
*values*, resolved by **fail-closed** classifiers (§0) that generalize the predicates
the current code already has (`secret` is *not* here — it is credential-extraction
intent, a positive per-*command* claim, §0):
`is_safe_write_target` (`src/cst/check.rs`) becomes `classify_locus`
over a write target (the same `/tmp`, `.git/`, `.envrc`, `$`-bearing, parent-escape
distinctions become `local` ordinal rungs); `PositionalShape`/`looks_like_path`
(`src/policy.rs`) becomes a shape input to the same classifier; `consumes_next_value`
(`src/policy.rs`) is unchanged — it is grammar, not facet, and still decides which
token is an argument. Operand roles (source vs sink for `rsync`/`scp`) flip the
dominant facet per v1.3 §3.3.

**Rule (§0, at the value level):** any value that cannot be pinned — `$VAR`, glob,
stdin, a substitution — takes the facet's worst term, and the assumption is recorded in
`because`. This is the profile-level statement of the allowlist floor already enforced
by denying unresolved substitutions (`src/cst/check.rs`), and the same fail-closed
posture §0 extends from unresolved *values* to unaddressed *facets*.

### 2.4 Modifier order

Modifiers must apply in a pinned order so the profile is a function of the form, not
of token order (v1.3 §3.5): `--dry-run` (→ all `observe`) first, then retargeting
(`-o FILE`/redirect → locus), then severity raisers (`--force` → `irreversible`, `-R`
→ `unbounded`), then injection-point flags. `--dry-run` dominating first is what makes
`terraform plan --destroy` observe-only regardless of the `--destroy` token.

## 3. Level evaluation

A level is a disjunction of **clauses**; each clause is a conjunction of per-facet
constraints plus a flow policy (v1.3 §4, §4.1). The TOML primitives are
`facet = "<= term"` (ordinal ceiling), `facet = ["a","b"]` (categorical set), and
`extends = "other"` (inherit clauses, then add).

Evaluation:

1. A **capability is admissible** iff some clause admits *every* one of its facets
   (ordinal ≤ ceiling; categorical ∈ set; compound facets checked axis-by-axis, see
   Findings). Clauses are tried in order; first admitting clause wins.
2. A **profile passes** iff every capability is admissible **and** the `Script`-scope
   flow policy holds.
3. The passing/failing profile is projected to a `Verdict`: pass → `Allowed(L')` where
   `L'` is the legacy-lattice projection of the profile (Stage-3 bridge run backward,
   for the reported level string only); fail → `Denied`. The gate is "profile passes
   active level," not the projected ordinal.

`extends` is clause-set union: `developer` inherits `write-local`'s clauses and adds
its supply-chain and outbound carve-ins (v1.3 §4.1). Compiling a level is the same
shape as `build_command` compiling TOML to a `DispatchKind` (`src/registry/build.rs`)
— a new `build_level` emitting a `Level` value of `Vec<Clause>`.

### 3.1 Resolution depth makes it lazy

The predicate names the resolution it needs (v1.3 §4.4). Facets are resolved
**lazily**: a level whose clauses never reference `execution.supply_chain` does not
descend a payload body; a level that ceilings `payload = forbid` decides at *presence*
of the raw hatch and never parses it. Unavailable resolution (no parser for the nested
language, an unpinnable argument) is worst-case → deny. This is the guarantee that the
engine need not become a universal interpreter: the deep case (a k8s `apply -f` body,
gate 4 — `behavioral-taxonomy-k8s-resolver`) is built once, per `(language, level)`,
and every level that does not demand it stops shallow.

## 4. Coexistence dispatch (v1.3 §4.5)

The converted-vs-legacy switch lives at the *top-level command lookup*, i.e. just
inside `simple_verdict` where `handlers::dispatch` is called today
(`src/cst/check.rs`), keyed on the command name exactly as `toml_dispatch` keys the
registry (`src/registry/mod.rs`):

- **Converted** (TOML carries a behavior profile) → resolve the profile (§2) and
  evaluate the active level (§3).
- **Legacy** (TOML carries only `Inert`/`SafeRead`/`SafeWrite`) → run the existing
  classifier unchanged, then map the legacy level onto the lattice via the Stage-3
  bridge — `Inert→inert`, `SafeRead→read-local`, `SafeWrite→developer` — and admit iff
  the mapped level lies within the active level.
- **Unclassified** (neither) → `Denied`, as today. The floor is unchanged.

A command's TOML signals converted-vs-legacy by the presence of profile data: the
natural encoding is a `[[command.profile]]`/`profile = "…"` presence check on
`TomlCommand` (mirroring how `handler`, `wrapper`, `deny` already select a build path
in `build_command`, `src/registry/build.rs`). Conversion is **atomic at the tool
subtree** (v1.3 §4.5): a command is converted only when every sub/flag/arg-shape under
it has profile data, enforced at build time by a `build_command` assertion (the same
class as the existing flat-vs-structured assertion) — no half-converted tool compiles.

Because the shipped default level is `developer` (≈ the old `SafeWrite` threshold),
converted and legacy commands auto-approve identically until a user opts into a
stricter level; the machinery is invisible until adopted.

## 5. The Stage-4 rollout harness

"Behind a flag" is a three-state selector — env var
`SAFE_CHAINS_ENGINE={legacy|shadow|new}` (or the equivalent CLI flag on the check
subcommand):

- `legacy` (default) — the current classifier is authoritative; the engine does not
  run.
- `shadow` — the legacy classifier is authoritative for the returned verdict, and the
  engine runs alongside, per level, recording old-vs-new divergences without affecting
  the decision.
- `new` — the engine is authoritative.

The corpus diff is a test/CI harness, not a runtime path: for each form in the
golden-set (v1.3 §5, the 65-form seed) and each command's `examples_safe` /
`examples_denied` (`src/registry/types.rs`), and for each shipped level, compute
`legacy_verdict` and `engine_verdict` and classify the divergence:

- **intended-tightening** — engine denies what legacy allowed, and the level's design
  says so (a `read-local` user holding out a `SafeWrite` tool). Logged, expected.
- **unacceptable-breakage** — engine denies what legacy allowed at the *default* level
  (`developer`), or allows what legacy denied. A gate; the build fails.

The harness is the operational form of v1.3 §4.5's "conversion is monotone and
reviewable": no command is worse off than its legacy behavior at the default level by
accident.

## 6. The proptest surface (v1.3 §4.2)

Represent the profile as one Rust enum per facet (ordinals as
`#[derive(PartialOrd, Ord)]` C-like enums, exactly as `SafetyLevel` `src/verdict.rs`;
categoricals as plain enums; compound facets as structs of their axes).
`Clause`/`Level` are `Vec`s of per-facet constraints. The existing proptest
infrastructure (type-directed `Arbitrary` strategies for words/scripts) is the model
for facet generation — add `Arbitrary` for `Capability` and `Profile`, and the
CST-refactor direction (type-directed generation over the parsed tree) supplies the
end-to-end generator.

The five contract properties become concrete tests:

- **facet-monotonicity** — for any admitted `Profile` and any single facet made *less*
  severe (ordinal down one rung / a categorical swapped for an admitted one), the
  profile stays admitted. A level failing this is incoherent by construction; the test
  is the coherence check on hand-authored level TOML.
- **extends ⇒ superset** — `admits(child, p)` ⇒ `admits(parent-extended, p)` for every
  generated `p`, over all `extends` pairs in the level set.
- **totality** — every generated `Profile` yields a decision (admit or deny), never a
  panic or "unknown"; the worst-case rule guarantees a total function.
- **round-trip** — `Level` → TOML → `Level` is identity, so the compiled artifact
  matches the source.
- **golden-set anchor** — the frozen 65-form corpus resolves to its recorded facet
  vectors; any taxonomy or engine change must keep it classifiable and re-review the
  diff (v1.3 §5).

## 7. Migration mechanics

Converting one command, end to end:

1. Replace the command's `level`/`DispatchKind` *data* with behavior-profile data in
   its `commands/*.toml` — each leaf's `level` becomes an explicit facet vector;
   delegating nodes declare their frame; argument-shaped facets declare their
   classifier. The grammar (subs, flags, `consumes_next_value`, positional shapes) is
   unchanged — it still decides *which tokens are what*.
2. The build-time atomicity assertion (§4) confirms the whole subtree carries profile
   data; the command flips to the engine automatically on next build — there is no
   registration step, matching the auto-discovery `build.rs` already does for TOML
   files.
3. Run the Stage-4 harness (§5): review the per-level golden-set diff, label each
   divergence, gate on zero unacceptable-breakage.

The load-bearing point: **the parsed CST is shared**. `command_verdict` parses once;
the `DispatchKind` walk and the profile resolver consume the same `tokens`/CST.
Conversion swaps the *classification* function at one leaf; every line above it (chain
semantics, substitution recursion, redirect handling, `eval` safety) is identical for
converted and legacy commands. This is what lets the corpus be a mix of the two for as
long as migration takes (v1.3 §6, Stage 5).

## Findings / risks

- **Compound facets are the representational sharp edge.** `locus{local,remote}` and
  `persistence{level, trigger{escape,kind}}` are not single ordinals, so admissibility
  is per-axis and monotonicity must hold *per axis* independently — a naive
  `derive(Ord)` on a compound struct will silently order `kernel` vs `remote` (the
  exact bug R25 fixed in the taxonomy). Model each axis as its own enum and write
  `admits` axis-by-axis; do **not** collapse a compound facet to one ordinal for
  convenience.
- **Modifier order-independence is a proptest liability.** §2.4 pins an order, but
  facet-monotonicity generation must not assume a canonical token order — generate
  profiles directly (post-modifier), not command strings, for the algebra tests, and
  test the modifier pipeline separately with a small hand-built set of `(--dry-run,
  --force, -R, redirect)` combinations to prove order-invariance of the *result*.
- **Shadow-compare cost is per-level × per-form.** Running the engine for every shipped
  level over the whole corpus on every CI run is O(levels × forms × resolution depth).
  Keep it a dedicated corpus-diff test, not part of the hot `command_verdict` path, and
  cache resolved profiles (profile resolution is independent of the level).
- **The projected verdict level is a lie of convenience.** Emitting `Allowed(L')` via
  the backward Stage-3 bridge keeps the fold and the reported level string working, but
  that ordinal is not the authoritative decision. Ensure nothing downstream (docs,
  `targets/*.rs` `additionalContext`) treats the projected `SafetyLevel` of a converted
  command as meaningful beyond display.
- **Flow policy escapes the leaf fold.** Because `secret`→outbound crosses pipe and
  substitution edges, it cannot be decided inside `simple_verdict`; it needs a
  `Script`-scope pass with access to the CST edges. Risk: getting it wrong *silently
  widens* (a missed edge admits an exfil). Ship it deny-only and default-conservative —
  an unrecognized channel is an outbound sink (v1.3 §2.5).

**First slice.** Convert **two flat, high-signal commands** that exercise the whole
pipeline without a deep grammar: (1) a pure-`Policy` read tool such as `grep` or `cat`
— additive leaf, argument-resolved `locus`/`disclosure`, no delegation, no modifiers;
it proves profile resolution, the `read-local` level, and the coexistence switch. (2)
`rm` (or a `WriteFlagged` command) — exercises the `--force`/`-R` modifier pipeline,
`reversibility`/`scale` worst-casing on globs and `$VAR`, and an `intended-tightening`
at `read-local`. Together they cover additive + modifier + argument-resolution + both
bridge directions with zero payload/flow depth, so the Stage-4 harness and the five
proptest properties can be stood up and proven green before any delegating or payload
command is touched.
