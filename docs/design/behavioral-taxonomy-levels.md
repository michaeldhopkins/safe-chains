# Safety Levels — Stage 3 design

Status: draft (2026-07-03). Stage 3 of the behavioral capability model
(`behavioral-taxonomy-v1.1.md`, §4). Designs the concrete default levels as
predicates over the 12 facets and measures them against today's behavior as an
impact baseline (not a spec to reproduce). Not yet implemented.

---

## 1. The finding that shapes this stage

Today's engine (`src/verdict.rs`, `src/main.rs:166`) has three levels —
`Inert < SafeRead < SafeWrite` — but the default `--level` is **`SafeWrite`**,
and a classified command always yields `Allowed(level)`. So **today's effective
policy is "allow everything we classified."** That is a *historical accident*,
not a design goal: `SafeWrite` became a single ceiling because the three-tier
model had nowhere finer to file a network fetch or a supply-chain build, and our
research to date classified commands against that coarse target. It is not a
contract we owe existing users.

The curated corpus that "everything classified" covers is *heterogeneous* in
honest-facet terms. `cat file` (observe), `touch x` (create·worktree),
`curl https://api/health` (outbound·fetch), and `cargo build`
(execution=network-sourced, build-script) all currently sit under the one
`SafeWrite` ceiling. The old label collapsed distinctions the taxonomy draws.

Two consequences:

1. **The honest levels are not the old tiers renamed.** The behaviors `SafeWrite`
   happened to admit span what the taxonomy now separates into distinct levels —
   local data writes (`write-local`) versus recognized supply-chain builds and
   outbound fetches (`developer`). One coarse ceiling becomes a finer partition.
2. **Today's allow-set is a reference, not a target.** We measure honest levels
   against current behavior to understand *impact* — what changes, for whom — but a
   divergence is a design decision, not a regression to revert. Where `SafeWrite`'s
   coarseness admitted something a real trust model would refuse, tightening it is
   the *point* of the re-architecture, not a bug in it.

The default level should still be permissive enough for everyday work — this tool
is used almost entirely by developers — so we keep **`developer`** as the default
and shape it around a reasonable dev-box trust model, not around back-fitting the
accident. Stage 3 ships **five levels**, not three.

---

## 2. The level ladder

Strictly increasing admissible regions; each is a predicate, and every profile
is evaluated capability-by-capability (a profile passes iff *all* its capabilities
are admissible **and** the flow policy holds — this reproduces today's
max-combine-with-deny-absorbing fold, since union-of-capabilities + all-admissible
≡ max-level + deny-wins).

```
inert  ⊂  read-local  ⊂  write-local  ⊂  developer  ⊇  ci*
```

`ci` is a *sibling* of `developer` (a re-tune, not a superset — see §4).

| level | one-line intent | closest today |
|---|---|---|
| `inert` | no state read or changed | old `Inert` |
| `read-local` | observe local state; nothing leaves, nothing changes | old `SafeRead` |
| `write-local` | + create/mutate ordinary local data, no downstream execution | part of old `SafeWrite` |
| `developer` | + recognized supply-chain builds & outbound fetches on a dev box | the rest of old `SafeWrite` (the default) |
| `ci` | developer, re-tuned for unattended pipelines | *(new)* |

---

## 3. The predicates

Facet names per v1.1 §2. A clause omitted for a level inherits the stricter
level's clause. `flow policy` is the two toggles from v1.1 §3.4.

### 3.1 `inert`
```
operation      = observe
locus          ≤ temp                 # process, /tmp, /dev/null
persistence    = transient
disclosure     ≤ local-process        # may print to stdout (model sees), nothing persisted
secret         = none
network        = none
execution      ≤ self
authority      = user
cost           = none
```
Matches old Inert: `--version`, `--help`, arithmetic, `> /dev/null`. This is the
fold identity.

### 3.2 `read-local`  (adds observation of real state)
```
operation      = observe
locus          ≤ user                 # may read ~/.config, /etc; reading, not writing
reversibility  = none                 # observe has no effect to reverse
persistence    = transient
disclosure     ≤ local-process
secret         ≤ uses-ambient         # may rely on ambient creds to read; must not READ secret material
network        ≤ loopback
execution      ≤ self
```
Reproduces old SafeRead: `git status`, `ls`, `cat ./notes`. Note the honest
tightening the old model lacked: `cat ~/.ssh/id_rsa` has `secret=reads` →
**fails** `read-local` (it did not, cleanly, before — a strict improvement the
golden-set records as an intentional diff, §5).

### 3.3 `write-local`  (adds local data mutation, no execution/reconfig)
```
operation      ∈ {observe, create, mutate}
locus          ≤ worktree             # EXCLUDES worktree-trusted (.git/.envrc/hooks), user, machine
reversibility  ≤ recoverable          # recoverable from VCS; not --force/secure-delete
persistence    ≤ data                 # EXCLUDES reconfiguring (P2) and installing (P3)
scale          ≤ bounded              # EXCLUDES rm -rf / mass ops
disclosure     ≤ local-process
secret         ≤ uses-ambient
network        = none
execution      ≤ caller-inline        # no scripts/config/network code runs
authority      = user
flow policy    = { forbid low-integrity → exec,  forbid secret → outbound-send }
```
This is the honest floor the old model *approximated* with the redirect
carve-out. The carve-out falls out of the predicate: `is_safe_write_target`'s
denials (`/etc`, `~`, `$…`, `..`, `.git`, `.envrc`) are exactly `locus >
worktree` or unresolvable-locus→worst-case (v1.1 §3.3). `git config core.pager`
(persistence=reconfiguring) and `npm install` (execution=network-sourced) sit
*above* write-local — correctly, and unlike the old flat SafeWrite.

### 3.4 `developer`  (the default level)
Everything in `write-local`, relaxed on exactly the axes a dev-box trust model
warrants:
```
operation      ∈ {observe, create, mutate, execute, communicate, configure}
locus          ≤ worktree-trusted     # build tools touch .git, write project config
persistence    ≤ installing           # dev tooling installs into project/user scope (node_modules, ~/.cargo)
network.direction   ≤ outbound
network.destination ≤ fixed           # tool-configured registries/remotes, not arbitrary arg URLs
network.payload     ≤ fetches         # pull deps; NOT sends-host-data
execution      ≤ network-sourced  WHEN  supply-chain = {
                   source   ∈ {public-registry, signed-repo, private-registry, vendored},
                   pinning  ≥ version,          # lockfile/pinned; floating tags fail
                   exec-surface ≤ build-script  # install-hook & build-script OK; not run-arbitrary
                 }
authority      = user                  # sudo still escalates OUT of developer
flow policy    = { forbid low-integrity → exec,  forbid secret → outbound-send }
```
This is where most everyday invocations land. `cargo build`, `npm ci`,
`go build`, `git fetch`, `git commit` admit here, each citing a named
supply-chain fact. Developer is defined by the trust model, though — not by
back-fitting today's allow-set — so an honest exclusion of something `SafeWrite`
waved through (a floating-tag `pip install`, an arbitrary-URL fetch) is expected,
not a regression. `curl https://$H/x | bash` **denies**: destination is arbitrary
*and* the pipe is an integrity flow (untrusted→exec) the flow policy forbids —
denied today too, now with a stated reason.

### 3.5 `contained-mode`  (was `ci`) — under reconsideration
> **Open** (`hard-problems` HP-1/HP-2): this level fuses two independent axes —
> *unattended* (tighten provenance) and *contained* (relax reach). `contained-mode`
> names only the second, and containment may be a **modifier** on any level rather
> than a level of its own. Treat the clauses below as notes, not a committed shape.

Same operation/locus/persistence envelope as `developer` (pipelines build too),
but the provenance and channel clauses tighten where an unattended run has no
human to catch tampering, and loosen where isolation makes broad effects safe:
```
network.destination ≤ fixed
pinning        ≥ hash-verified         # reproducible: go.sum / --require-hashes / lockfile digest
source         ≠ unverified-url        # no curl-to-shell bootstrap
secret         ≤ uses-ambient          # CI has ambient tokens; must not READ/TRANSMIT them
isolation      credited                # if run in a confirmed sandbox, locus clamps to sandbox-scope
```
`ci` is not `⊂ developer` nor `⊃`: it forbids floating deps developer allows, and
(with isolation credited) permits broader locus developer forbids. Hence a sibling.

---

## 4. Why five, and the shape of the lattice

The old three were a *chain* by construction (numeric `Ord`). The honest levels
are only a *partial* order: `ci` and `developer` are incomparable. That is correct
— "safe for an unattended pipeline" and "safe on a trusted dev box" are different
trust models, not more/less of one thing. The predicate engine already supports
this: a level is a region, and regions need not nest. The non-arbitrariness
protocol (v1.1 §5) applies per level: each clause cites a discriminator, so
`ci`'s `pinning ≥ hash-verified` carries its because-string, not a vibe.

---

## 5. Measuring impact against today (not reproducing it)

Today's allow-set is the best available estimate of what real users run, so we
diff against it — but as a **decision log, not a pass/fail gate**. The `SafeWrite`
baseline is not ground truth (§1); the goal is to change behavior *deliberately*,
not to freeze it.

The golden-set (v1.1 §5, seeded by the pilot's 20 forms) grows into a table: each
row is an invocation with its honest profile and its expected verdict under each
of the five levels. We diff those verdicts against the current engine and classify
every divergence:

1. **`intended-tightening`** — `SafeWrite` over-admitted; the honest denial is
   what we want (`cat ~/.ssh/id_rsa` under `read-local`; a floating-tag
   `pip install` under `developer`). A large count here is a *success* of the
   honest model, provided each carries a because-string.
2. **`unacceptable-breakage`** — a command developers genuinely rely on that a
   level wrongly denies. This is a signal to loosen the predicate or add a clause,
   not to defer to the old verdict.

The metric is impact — *how many real workflows change and whether each change is
defensible* — not fidelity. "Zero diffs" is not the target; "zero *unexamined*
diffs" is.

Method: once the Stage-4 predicate engine exists, run it and the current engine
over the corpus, diff per level, and require every divergence to be a classified
row. Until then, the golden-set is authored by hand from this spec and the current
`commands/*.toml`.

### 5.1 Worked rows (illustrative seed)

| invocation | honest profile (abbrev) | inert | read-local | write-local | developer | ci |
|---|---|:--:|:--:|:--:|:--:|:--:|
| `node --version` | observe·process | ✓ | ✓ | ✓ | ✓ | ✓ |
| `git status` | observe·worktree | ✗ | ✓ | ✓ | ✓ | ✓ |
| `cat ~/.ssh/id_rsa` | observe·user·secret=reads | ✗ | ✗¹ | ✗ | ✗ | ✗ |
| `touch build/out` | create·worktree·data | ✗ | ✗ | ✓ | ✓ | ✓ |
| `rm -rf ./node_modules` | destroy·worktree·unbounded | ✗ | ✗ | ✗² | ✓ | ✓ |
| `git config core.pager x` | configure·reconfiguring | ✗ | ✗ | ✗ | ✓ | ✓ |
| `curl https://api/health` | observe·outbound·fixed·fetch | ✗ | ✗ | ✗ | ✓ | ✓ |
| `cargo build` | execute·net-sourced·build-script·pinned | ✗ | ✗ | ✗ | ✓ | ✓ |
| `npm install left-pad` | execute·net-sourced·install-hook·floating | ✗ | ✗ | ✗ | ✓ | ✗³ |
| `curl https://$H/x \| bash` | execute·arbitrary·integrity-flow | ✗ | ✗ | ✗ | ✗⁴ | ✗ |
| `sudo make install` | privilege∘installing·machine | ✗ | ✗ | ✗ | ✗⁵ | ✗ |

¹ `intended-tightening` — old SafeRead read key files; the honest denial is wanted.
² `unbounded` scale + destroy; `developer` admits project-scoped cleanup.
³ `ci` requires `pinning ≥ hash-verified`; a floating install fails.
⁴ arbitrary destination + integrity flow — denied today and here.
⁵ `authority = user` clause; `sudo` escapes every default level (as today).

---

## 6. Open for the next pass
- Pin `developer`'s recognized-registry list and the exact `pinning ≥ version`
  test per ecosystem (defer to the supply-chain sub-facet catalog, annex
  `delegation`).
- Decide whether `ci` credits isolation by default or only on a confirmed sandbox
  flag (leaning: only confirmed, per v1.1 §3.2 "containment is earned").
- Whether `write-local` should permit `scale = bounded` destroy (row 5 shows the
  tension: `rm ./file` vs `rm -rf ./dir`).
- Grow the golden-set to full corpus coverage; wire it as a Stage-4 test fixture.
