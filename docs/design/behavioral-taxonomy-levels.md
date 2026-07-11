# Safety Levels — Stage 3 design

Status: draft (2026-07-03; level set revised 2026-07-07). Stage 3 of the behavioral
capability model (`behavioral-taxonomy-v1.1.md`, §4). Designs the concrete default
levels as predicates over the 12 facets and measures them against today's behavior as
an impact baseline (not a spec to reproduce). The default ladder is now authored in
`levels/default.toml` and enforced by the engine — see §0 for the finalized names and shape.

> **Revised (2026-07-07, `…-refinements` §5–6, canon `v1.4` §4.3):** the level set is
> now `inert ⊂ read-local ⊂ write-local ⊂ developer ⊂ yolo`, with `admin`/`infra` as
> deny-by-default siblings. The **`ci`** level below is **retired** — it would never be
> selected in a human-in-the-loop hook; its provenance-strictness became the opt-in
> `pinned-provenance` *modifier*. **`yolo`** (§3.6) is the new opt-in top of the local
> ladder. Read §3.5 as historical.

---

## 0. Finalized level model (2026-07-15)

§2–3 remain authoritative on the *facet predicates*. This section fixes the **names and
shape** decided 2026-07-15 and implemented in `levels/default.toml`. Levels are meant to
be plain promises a human can choose between, so the engine-internal names are renamed to
human ones: `inert → paranoid`, `read-local → reader`, `write-local → editor`;
`developer`/`yolo` keep their names; the deny-by-default siblings `admin`/`infra` become
first-class **`local-admin`** / **`network-admin`**.

The ladder, locked → open:

```
paranoid → reader → editor → developer ─┬─→ local-admin ─┐
                                        └─→ network-admin ┴─→ yolo
```

| Level | The promise a person is choosing |
|---|---|
| **paranoid** | Barely touches anything — `--version`, `--help`, arithmetic. Doesn't even open your files. The floor; the opposite pole from yolo. |
| **reader** | Reads state — local *and* remote alike: files/listings/`git status`, plus a pure remote fetch (`curl` GET, `koyeb apps list`). A read is a read wherever the data lives; only paranoid blocks the network. Changes nothing, sends no host data out (exfil stays above it). |
| **editor** | Creates and edits your files, but won't delete, run code, or hit the network — so nothing it does is hard to undo. |
| **developer** | Runs your project and its pinned deps, edits/deletes your *own* files, uses your everyday tools. Stops short of anything stupidly destructive, privileged, or off-machine. |
| **local-admin** | Developer + trusted to run *this machine*: `sudo`, services, `/etc`, mounts, system installs. Never reaches the network. |
| **network-admin** | Developer + trusted to operate *your remotes*: deploy, push, provision, cloud APIs. Never `sudo`s the box. |
| **yolo** | No limits. For throwaway VMs and living dangerously. |

**Why the two flavors are *siblings*, not rungs.** A level is a predicate over
capability-space, so two levels can be **incomparable** — neither more nor less
permissive, opening *different* facet regions. local-admin flexes DOWN into this host
(`authority`, `locus.local`, install `persistence`); network-admin flexes OUT to other
hosts (`locus.remote`, `network`, `cost`). They light up disjoint columns:

| axis | developer | **local-admin** | **network-admin** |
|---|---|---|---|
| authority | user | **≤ root** | user |
| locus.local | ≤ worktree-trusted | **≤ device** | ≤ worktree-trusted |
| locus.remote | none | none | **≤ arbitrary** |
| network | none | none | **≤ outbound** |
| persistence | ≤ data | **≤ installing** | ≤ reconfiguring |
| cost | none | local-resource | **≤ quota** |

This is precisely what a linear `Inert < SafeRead < SafeWrite` enum could not represent: a
total order has no room for siblings. Offering two directions of extra permission *at the
same trust tier* is a capability the facet model unlocks.

**The reversibility spine.** Every level *below* yolo caps destruction at `reversibility
≤ effortful` (recoverable with real work). **Only yolo lifts it to `irreversible`.** So
`destroy · irreversible` — `terraform destroy`, `aws rds delete --skip-final-snapshot`,
`mkfs`, `shred`, `rm -rf` on irreplaceable data — is reserved for yolo, on *any* locus,
local or remote. One shared cap, inherited by both admin flavors, keeps permanent
no-recovery-path destruction out of everything but the extreme option. (This converges
with v1.4 §4.3's existing `infra: reversibility ≤ effortful`.) An *additive* irreversible
like `npm publish` is a separate, lesser case — the reservation is specifically `destroy ·
irreversible`, not irreversibility categorically.

**Ceilings held back to yolo** from the two flavors: kernel-module load (`locus > device`),
setuid / run-as-another-user (`authority > root`), inbound-listen servers, public
disclosure, credential transmission, and network-sourced execution (a pulled container
image — that awaits the supply-chain clause).

**Mapping to the current 3-value ceiling.** The CLI still selects a threshold from the
legacy `SafetyLevel` enum (`inert` / `safe-read` / `safe-write`). Until threshold-by-name
lands, the engine (`bridge::project`) maps `paranoid → inert`, `reader → safe-read`,
`editor` and `developer → safe-write`, and **`local-admin` / `network-admin` / `yolo →
Denied`** — no legacy equivalent, so a command needing an upper level is never
auto-approved (guarded by `bridge::profiles_needing_an_upper_level_project_to_denied_not_safewrite`).
Making the upper levels threshold-selectable is the follow-up; the model itself is authored
and enforced today.

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
accident. Stage 3 ships a **finer partition** than the old three — the nesting ladder
`inert ⊂ read-local ⊂ write-local ⊂ developer ⊂ yolo` plus the `admin`/`infra` siblings
(see the top note for the revised set).

---

## 2. The level ladder

Strictly increasing admissible regions; each is a predicate, and every profile
is evaluated capability-by-capability (a profile passes iff *all* its capabilities
are admissible **and** the flow policy holds — this reproduces today's
max-combine-with-deny-absorbing fold, since union-of-capabilities + all-admissible
≡ max-level + deny-wins).

```
inert  ⊂  read-local  ⊂  write-local  ⊂  developer  ⊂  yolo        (+ admin, infra siblings)
```

(Revised set — see the top note. `yolo` is the opt-in loosest local level, §3.6; `ci`
below is retired.)

| level | one-line intent | closest today |
|---|---|---|
| `inert` | no state read or changed | old `Inert` |
| `read-local` | observe local state; nothing leaves, nothing changes | old `SafeRead` |
| `write-local` | + create/mutate ordinary local data, no downstream execution | part of old `SafeWrite` |
| `developer` | + recognized supply-chain builds & outbound fetches on a dev box | the rest of old `SafeWrite` (the default) |
| `yolo` | + anything local short of the irrecoverable (opt-in) | *(new)* |
| `ci` | ~~developer, re-tuned for unattended pipelines~~ — **retired** → `pinned-provenance` modifier | *(removed)* |

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

### 3.2 `read-local`  (adds observation of worktree state)
```
operation      = observe
locus.local    ≤ worktree-trusted     # worktree + .git reads; NOT home/absolute content
reversibility  = none                 # observe has no effect to reverse
persistence    = transient
disclosure     ≤ local-process
secret         ≤ uses-ambient         # may rely on ambient creds; credential-EXTRACTION (secret=reads) fails
network        ≤ loopback
execution      ≤ self
```
Reproduces old SafeRead's worktree cases: `git status`, `ls`, `cat ./notes`,
`grep -r foo src/`. The honest tightening (fail-closed, §0 of annex `…-engine`):
content reads are bounded by **locus**, so `cat ~/.ssh/id_rsa` **fails** `read-local`
because it is a `user`-scope content read to the model — *not* because `id_rsa` was
detected as secret (that would be a denylist). The same bound denies the
unanticipated `cat ~/.config/newtool/token`; a worktree read a level trusts
(`./.env` ≡ `./notes.md`) is a named residual risk, not a list gap. `secret ≤
uses-ambient` now excludes credential-*extraction* commands (keychain). The golden-set
records the home-read tightening as an intentional diff (§5).

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

The `operation` line is the write-local ↔ developer boundary: **create/mutate**
(including an *overwrite* — `echo > config.json`, `cp ./a ./b`, whose reversibility
is `recoverable` under the repo-recoverable assumption) is write-local, while
**destroy** (`rm`, `find -delete`) waits for `developer`. The split is by operation,
not reversibility — deletion is held one level higher as a matter of policy (golden-set
§3, decision 2), even though a tracked file is itself recoverable.

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
                   pinning  ≥ floating,         # NO pinning floor — floating installs OK (golden-set §5.3)
                   exec-surface ≤ build-script  # install-hook & build-script OK; not run-arbitrary
                 }
authority      = user                  # sudo still escalates OUT of developer
flow policy    = { forbid low-integrity → exec,  forbid secret → outbound-send }
```
This is where most everyday invocations land. `cargo build`, `npm ci`,
`go build`, `git fetch`, `git commit` admit here, each citing a named
supply-chain fact. Developer gates on **source and exec-surface, not pinning**: a
floating `npm install left-pad` / `pip install requests` admits (golden-set §5.3 — the
source is a recognized registry), while an **arbitrary-URL fetch** does not. The opt-in
`pinned-provenance` modifier is what adds a pinning floor for users who want it. `curl
https://$H/x | bash` **denies**: source is `unverified-url` *and* the pipe is an
integrity flow (untrusted→exec) the flow policy forbids — denied today too, now with a
stated reason.

### 3.5 `ci`  (was `contained-mode`) — retired
> **Retired as a level** (`behavioral-taxonomy-refinements` §5, HP-1/HP-2). This fused
> two independent axes; *both* resolved to **modifiers, not levels**. *Contained* → the
> **isolation modifier** (a sandbox transform on the profile, §3.2). *Unattended* → the
> opt-in **`pinned-provenance` modifier**: safe-chains is always human-in-the-loop, so a
> stricter "unattended pipeline" *level* would never be selected — its one durable idea
> (tighter provenance) is a preference knob dialed onto whatever level is active. The
> clauses below are kept as the historical statement of that provenance-tightening; read
> them as the modifier's content, not a level.

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

### 3.6 `yolo`  (opt-in loosest local level)
The top of the *local* ladder, `developer ⊂ yolo`, opted into per-environment (the
`admin`/`infra` trusted-config gate). Intent: "do anything to a machine I own or can
throw away — `sudo`, `rm`, installs — short of the irrecoverable." It is the first level
whose natural shape is **allow-almost-everything minus a few corners**, so it is the sole
client of the level language's bounded, allow-only `deny` clause: a maximal *local* allow
(any operation, up to `root`, `locus.local ≤ machine`, any scale, `remote ≤ fetch-only`,
`destination ≤ arbitrary`, `payload ≤ fetches`) with five **catastrophe corners**
subtracted —
```
deny  destroy ∧ irreversible ∧ scale ≥ machine-wide      # C1  mkfs, dd of=/dev/sda, rm -rf ~
deny  (destroy|mutate) ∧ irreversible ∧ locus.local ≥ machine   # C1b overwrite a system path / whole fs
deny  execute ∧ authority = root ∧ source = unverified-url      # C3  curl | sudo bash
# C2 (device/kernel), C4 (remote mutation), C5 (secret→chat/external) fall out of scoping
# the positive allow — they are simply never granted.
```
`deny` runs after the allow and can only *remove* capability (monotonic-downward), so it
cannot forge a stricter level from a looser base — the R27 worry runs the other way.
Full design + proptests in `…-refinements` §6. `yolo ⊃ developer` and `⊃ (admin ∩
¬catastrophe)`; `infra` (remote mutation) stays outside it by C4.

---

## 4. Why not a single chain — the shape of the lattice

The old three were a *chain* by construction (numeric `Ord`). The honest levels are
only a *partial* order: the local ladder `inert ⊂ read-local ⊂ write-local ⊂ developer
⊂ yolo` nests, but `admin` (local root) and `infra` (remote cloud) are **siblings, mutually
incomparable** — "safe to administer this box" and "safe to operate a cloud" are different
trust models, not more/less of one thing (a laptop enables `admin` and never `infra`; a CI
runner the reverse). The predicate engine already supports this: a level is a region, and
regions need not nest. The non-arbitrariness protocol (v1.1 §5) applies per level: each
clause cites a discriminator, so `admin`'s "no `device`/`kernel`" and `yolo`'s catastrophe
`deny` clauses each carry their because-string, not a vibe.

---

## 5. Measuring impact against today (not reproducing it)

Today's allow-set is the best available estimate of what real users run, so we
diff against it — but as a **decision log, not a pass/fail gate**. The `SafeWrite`
baseline is not ground truth (§1); the goal is to change behavior *deliberately*,
not to freeze it.

The golden-set (v1.1 §5, seeded by the pilot's 20 forms) grows into a table: each
row is an invocation with its honest profile and its expected verdict under each
level. We diff those verdicts against the current engine and classify
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

| invocation | honest profile (abbrev) | inert | read-local | write-local | developer | yolo |
|---|---|:--:|:--:|:--:|:--:|:--:|
| `node --version` | observe·process | ✓ | ✓ | ✓ | ✓ | ✓ |
| `git status` | observe·worktree-trusted | ✗ | ✓ | ✓ | ✓ | ✓ |
| `cat ./notes.md` · `cat ./.env` | observe·worktree·→model | ✗ | ✓ | ✓ | ✓ | ✓ |
| `cat ~/.ssh/id_rsa` | observe·**user**·→model | ✗ | ✗¹ | ✗ | ✗ | ✗² |
| `touch build/out` | create·worktree·data | ✗ | ✗ | ✓ | ✓ | ✓ |
| `rm -rf ./node_modules` | destroy·worktree·bounded·recoverable | ✗ | ✗ | ✗³ | ✓ | ✓ |
| `git config core.pager x` | configure·reconfiguring | ✗ | ✗ | ✗ | ✓ | ✓ |
| `curl https://api/health` | observe·outbound·fixed·fetch | ✗ | ✗ | ✗ | ✓ | ✓ |
| `cargo build` | execute·net-sourced·build-script·pinned | ✗ | ✗ | ✗ | ✓ | ✓ |
| `npm install left-pad` | execute·net-sourced·install-hook·floating | ✗ | ✗ | ✗ | ✓ | ✓⁴ |
| `curl https://$H/x \| bash` | execute·arbitrary·integrity-flow·user | ✗ | ✗ | ✗ | ✗⁵ | ✓⁶ |
| `sudo make install` | privilege∘installing·machine·local-source | ✗ | ✗ | ✗ | ✗⁷ | ✓⁸ |
| `curl https://$H/x \| sudo bash` | execute·unverified-url·**root** | ✗ | ✗ | ✗ | ✗ | ✗⁹ |
| `dd of=/dev/sda …` · `mkfs …` | destroy·device·irreversible | ✗ | ✗ | ✗ | ✗ | ✗¹⁰ |
| `git push` | mutate·remote | ✗ | ✗ | ✗ | ·† | ✗¹¹ |

¹ `intended-tightening` — denied by **locus** (a `user`-scope content read to the model),
  not by detecting `id_rsa` as secret (§0 fail-closed). `./notes.md` and `./.env` (both
  worktree) are admitted alike — a named residual risk, not a list gap.
² Denied by locus+disclosure even at `yolo`: content-to-model from beyond the worktree is
  egress to the model provider, outside `yolo`'s *local* license (the fail-closed
  successor to the old secret→chat corner — no file is *detected* as secret).
³ `bounded` worktree destroy waits for `developer`; `write-local` doesn't auto-delete.
⁴ `yolo` allows floating installs — looser than `developer`, and not a catastrophe corner.
⁵ arbitrary destination + integrity flow — denied at `developer` (and today).
⁶ C-none: run as the *user* — `yolo` opts out of the integrity-flow forbid at user level
  (that is the level's whole point). The root form (next row) stays denied.
⁷ `authority = user` clause; `sudo` escapes `developer` (as today).
⁸ `yolo` folds in `admin`'s root grants; the install code's source is the local project,
  not an unverified URL (C3), so it runs.
⁹ C3: unverified-URL code executed **as root** — denied even at `yolo`.
¹⁰ C1/C2: irreversible device-level destruction — the catastrophe floor, never auto.
¹¹ C4: leaves the machine — remote mutation is `infra`, outside `yolo`'s local license.

---

## 6. Open for the next pass
- Pin `developer`'s recognized-registry list and the exact `pinning ≥ version`
  test per ecosystem (defer to the supply-chain sub-facet catalog, annex
  `delegation`).
- Confirm the exact boundary of `yolo`'s integrity-flow relaxation: user-level exec of
  downloaded code is allowed (row 6), root-level is not (C3, row 9) — is that the whole
  rule, or are there user-level cases (e.g. writing an auto-run hook) that should still ask?
- Whether `write-local` should permit `scale = bounded` destroy (the `rm -rf ./node_modules`
  row shows the tension: `rm ./file` vs `rm -rf ./dir`).
- Golden-set is now full-corpus (§`…-golden-set`); remaining work is to **freeze** it and
  wire it as a Stage-4 test fixture, including a `yolo` column.
