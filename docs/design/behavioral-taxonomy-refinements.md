# Pinning three evidenced pieces — trigger axis, sub-machine loci, infra level

Status: refinement (2026-07-04). Pins three pieces v1.2 introduced but left
under-specified: the Persistence **trigger** sub-axis (R16), the sub-filesystem
**loci** `device`/`kernel` (R18), and the **`infra`** level (R12). Each gets terms,
discriminators, examples, and a level-treatment mapping. Pinning them surfaced three
new findings (R24–R26) — two of them say the v1.2 shape is slightly wrong. Feeds a
v1.3 and `behavioral-taxonomy-levels`.

---

## 1. The Persistence trigger sub-axis

**What it captures.** Given an effect that outlives the command, *when and how often
does it fire, relative to the check?* The check happens once; the trigger says how
far execution escapes that moment.

**Terms, discriminators, examples.**

| term | discriminator (vs neighbors) | examples |
|---|---|---|
| `immediate` | fires during the command; complete on return; no future firing | `touch x`, `rm x`, `git commit` |
| `detached` | one instance keeps running past the session, but does **not** re-fire | `nohup ./srv &`, `setsid`, `screen -dm`, a spawned daemon |
| `scheduled` | re-fires at future **clock** times, no process kept alive | `crontab`, `at`, `systemd-run --on-calendar`, launchd `StartCalendarInterval` |
| `event` | re-fires on an **event** (fs change, cd, a git op), while the watcher/hook exists | `watchexec`, `entr`, a git hook, `.envrc` (fires on cd) |
| `boot` | re-fires at **startup/login**, surviving reboot | `systemctl enable`, launchd `RunAtLoad`, `@reboot`, login items, `~/.zshrc` |

**R24 — trigger is not one clean ordinal; it is escape-severity + kind.** v1.2 listed
`immediate | scheduled | event | boot | detached` as if increasing, but `scheduled`
and `event` are not more/less severe than each other — a per-save `event` trigger can
fire far more than a monthly `scheduled` one. The severity-bearing property is *how
far execution escapes the check*, which decomposes:
- **escape ordinal** (this is the ordinal levels gate on): `immediate` < `detached`
  (escapes the session, one instance) < `recurring` (re-fires until removed) < `boot`
  (re-fires and survives reboot — a permanent foothold).
- **kind** (categorical, sits under `recurring`): `clock` (scheduled) vs `event`.
  Distinguishes *what* fires it, for the `because` string; not a severity rung.

So the pinned shape is: `trigger.escape ∈ {immediate < detached < recurring < boot}`
with `trigger.kind ∈ {clock, event}` populated when `escape = recurring`.

**Independence from Persistence level.** Trigger is genuinely orthogonal to the
level (transient/data/reconfiguring/installing). `nohup sleep 1000 &` installs no
durable artifact (`persistence.level = transient`) yet escapes the session
(`trigger.escape = detached`) — a dimension the level facet misses. `crontab` is
`installing · scheduled(recurring)`. Confirms trigger earns its own sub-axis.

**Level treatment (the knob it buys).**

| level | `trigger.escape ≤` | rationale |
|---|---|---|
| inert / read-local / write-local | `immediate` | no deferred or recurring execution at all |
| developer | `detached` | background a dev server (`npm run dev &`), but not schedule/persist |
| infra | `boot` | installing services / cron / startup units is infra work (§3) |

That ladder is clean and useful: "may this command arrange for code to run when
I'm not watching?" is exactly the question levels want to answer, and the escape
ordinal answers it monotonically.

---

## 2. Sub-filesystem loci: `device`, `kernel`

**Definitions, discriminators, examples.**

- **`device`** — operates on a raw block/character device or the mount namespace,
  *beneath* the filesystem abstraction. Discriminator: bypasses filesystem
  permissions and structure — writes/reads the device directly, or changes what the
  fs namespace contains. Positive: `dd of=/dev/rdisk0`, `diskutil eraseDisk`,
  `parted`, `mount`/`umount`, `hdiutil attach x.dmg`. Near-miss: `dd of=./file` is
  `worktree` (needs a device target); `echo > /dev/null` is `temp` (special file,
  not raw storage).
- **`kernel`** — introduces code or persistent configuration into the kernel (ring 0).
  Discriminator: kernel-resident code/state, not a userspace effect. Positive:
  `kmutil load`/`kextload`, `insmod`/`modprobe`, loading a BPF program. Near-miss:
  `sysctl -w kernel.x=…` is `machine · configure` (a runtime param, recoverable), not
  `kernel` — kernel is reserved for *code/module* load.

**R25 — locus is really two axes; `device`/`kernel` expose it.** v1.2's single
ordinal `… machine → device → kernel → remote` forces a false comparison: `kernel`
(ring-0, this host, roots the box) and `remote` (another host) are not more/less than
each other — they are different *places*. The honest shape is:
- **local depth** (ordinal): `process < temp < sandbox-scope < worktree <
  worktree-trusted < user < machine < device < kernel`.
- **remote reach** (separate): a `remote` flag carrying its own destination
  classification — which host / trust domain — the same axis as Network.destination
  and the ambient-target problem (HP-12).

A level predicate then reads `locus.local ≤ worktree ∧ locus.remote = none` instead of
pretending kernel and remote lie on one line. This also gives remote-locus a place to
carry the pinned-vs-ambient distinction infra needs (§3).

**Level treatment: `device`/`kernel` are deny-by-default everywhere.** No shipped
level (including `infra`) auto-approves them; they void the abstractions every other
locus assumes, and there is no routine workflow that needs `dd` to a raw disk or a
kext load un-prompted. They require an explicit, hand-authored per-command allowance
or a user's deliberately-permissive level. This is a clean stance, not a gap:

| level | `locus.local ≤` | `locus.remote` |
|---|---|---|
| inert | `temp` | none |
| read-local | `user` (observe only) | none |
| write-local | `worktree` | none |
| developer | `worktree-trusted` | fetch-only (fixed dest) |
| infra | `machine` (see R26) | mutate, pinned dest |
| *(any shipped)* | never `device`/`kernel` | — |

---

## 3. The `infra` level

**R26 — "infra" is at least two trust models.** Pinning it shows R12 lumped two
distinct models that a user may trust independently:
- **remote-cloud-operator** — mutate cloud/cluster state: `terraform apply`, `aws …
  create/delete`, `kubectl apply`, `helm`, `gcloud`. Remote locus, metered, shared
  blast radius, ambient credentials.
- **local-privileged-admin** — mutate the local machine as root: `sudo apt install`,
  `systemctl`, editing `/etc`, `brew` into system paths. `machine` locus, `root`
  authority, `installing`.

A laptop dev does the second and never the first; a platform pipeline does the first
on an ephemeral runner and never touches a human's `/etc`. They are different trust
grants. We define **`infra` = remote-cloud-operator** here and leave
**`admin` = local-privileged** as a named sibling (follow-up).

**The `infra` predicate.**

```toml
[level.infra]                        # remote-cloud operator; deny-by-default, opt-in
extends = "developer"

[[level.infra.allow]]                # provision / mutate remote infrastructure
operation     = ["create", "mutate", "control", "configure"]
locus         = { remote = "pinned" }   # R25: named on the command line, NOT ambient
reversibility = "<= effortful"          # excludes irreversible remote destroy -> prompts
cost          = "<= quota"              # money / rate-limit allowed
network       = { direction = "<= outbound", destination = "<= fixed",
                  payload = "<= sends-host-data" }
secret        = "<= uses-ambient"
trigger       = "<= boot"              # install services / scheduled jobs (§1)

[[level.infra.allow]]                # apply a trusted-source declarative payload
delegate = "payload"
verb     = ["apply", "plan", "diff", "get"]
source   = "<= worktree-trusted"        # in-repo manifests/HCL only
# gate-3/4 per behavioral-taxonomy-payload-survey; k8s R3 available, others R1/R2
```

**Three deliberate exclusions, each citing a fact:**
1. **No ambient target.** `locus.remote = "pinned"` operationalizes HP-12: `infra`
   admits a remote mutation only when the target is named on the command line
   (`--context prod`, `--profile`, explicit host). An unpinned `kubectl apply` whose
   cluster comes from ambient state resolves `remote = ambient` → worst-case → not
   admitted → prompt. This is the concrete level-side answer HP-12 asked for.
2. **No irreversible remote destroy.** `reversibility ≤ effortful` lets `infra`
   auto-approve routine provisioning (create/apply/scale) but leaves `terraform
   destroy` / `kubectl delete namespace prod` (irreversible) to a human prompt — the
   scariest ops stay gated even for an operator.
3. **No local root, no device/kernel.** `authority` stays `user` (cloud auth is
   ambient creds, not `sudo`); local machine/root is the `admin` sibling; `device`/
   `kernel` are excluded everywhere (§2).

`infra` is **not** in the default threshold. A user opts in per-directory/per-session
through the v0.205.0 trusted-config model, exactly where a repo that *is* an infra
repo would declare it.

---

## 4. The `admin` level (the local-privileged sibling)

`admin` is the trust model `infra` split away from: **administering *this machine* as
root**. Where `infra` is remote + ambient-cloud-auth, `admin` is local + `sudo`. It
extends `developer` (a root context can do everything a dev box can) and adds
machine-scoped, elevated operations.

```toml
[level.admin]                          # local privileged sysadmin; deny-by-default, opt-in
extends = "developer"

[[level.admin.allow]]                  # write / reconfigure / control the local machine as root
operation     = ["create", "mutate", "destroy", "configure", "control"]
locus         = { local = "<= machine", remote = "none" }   # /etc, services, system paths
authority     = "<= root"                                   # sudo/doas (developer caps at user)
persistence   = "<= installing"
reversibility = "<= effortful"                              # irreversible wipes still prompt
scale         = "<= bounded"                                # unbounded destroy still prompts

[[level.admin.allow]]                  # system package managers (apt/dnf/pacman/brew) as root
operation    = ["execute"]
execution    = "<= network-sourced"
authority    = "<= root"
supply_chain = { source = ["distro-repo", "public-registry", "signed-repo"],
                 pinning = ">= version", exec_surface = "<= install-hook" }
network      = { direction = "<= outbound", destination = "<= fixed", payload = "<= fetches" }
flow         = { low_integrity_exec = "forbid", secret_outbound = "forbid" }
```

**What it admits, and the honest line it draws.** `admin` accepts the corpus's worst
combination on purpose: **root supply-chain execution** — `sudo apt install nginx`
runs the maintainer's `postinst` as root (pilot-2). That *is* system administration.
But it is bounded: `source ∈ {distro-repo, registry, signed-repo}`, `pinning ≥
version`, `exec-surface ≤ install-hook`, `destination = fixed`. So *install a signed
package as root, yes; run a downloaded script as root, no* — `curl x | sudo bash`
still fails, because the destination is arbitrary **and** the flow policy forbids
low-integrity→exec regardless of authority. Root does not relax the flow doctrine.

**Four fact-cited exclusions:**
1. **No `device`/`kernel`.** `dd of=/dev/rdisk0`, `kmutil load` stay deny-everywhere
   (§2) — routine admin never needs raw-disk or ring-0 un-prompted.
2. **No unbounded destroy.** `sudo rm -rf /`, disk wipes (`scale = unbounded`) still
   prompt even at root.
3. **No remote.** Cloud mutation is `infra`; `admin.remote = none`. Administering a
   box and operating a cloud are different grants.
4. **No arbitrary-source supply chain.** As above — the `curl | sudo bash` class.

`admin` and `infra` are **siblings, incomparable**: local-root vs remote-cloud. A
laptop enables `admin` (`brew`/`apt`/`systemctl`) and never `infra`; a CI runner
enables `infra` and never a human's `/etc`; a platform box enables both. Both are
deny-by-default, opt-in through the trusted-config model. (`brew` without `sudo`
installs into a user-writable prefix — that stays `developer`; the `admin`-only cases
are the ones that need `sudo` / touch `/etc` / control services.)

---

## 5. Containment is a *modifier*, not a level (resolving HP-1 & HP-2)

The `contained-mode` level (née `ci`) was a mis-modeling. Pinning it the way we
pinned `infra` shows it fused **two orthogonal axes** (HP-1), which resolve to two
*different kinds of thing*:

- **contained** — a confirmed sandbox bounds blast radius → *relax* reach.
- **unattended** — no human to catch tampering → *tighten* provenance.

**Contained → a modifier (this is HP-2's answer).** A sandbox transforms the
**profile**, not the predicate — and the isolation mechanism (§3.2) already does
exactly that: it clamps nested `locus` to `sandbox-scope`, caps `reversibility` to
`recoverable` (a sandbox is disposable), and re-adds breach loci on `-v /:/host` /
`--privileged` / `--pid=host`. So "`developer`-in-a-sandbox admits more than
`developer`" is **not a new level** — it is the *same* `developer` predicate
evaluating a profile the isolation modifier has already tamed. Because the modifier
runs *before* whatever predicate is active, containment composes with **every** level
for free (this settles the refinements "still open": yes, an `infra` operator inside a
confirmed CI sandbox is just `infra` evaluated against a sandbox-clamped profile).
`contained-mode` is therefore **retired as a level** — it is subsumed by §3.2.

**Unattended → an optional *modifier*, not a level (`ci` retired).** This axis
looked like it changed the predicate — with no human watching, tolerate only tighter
provenance — so it was first modeled as a stricter `developer` level, `ci`. But
safe-chains runs as a **human-in-the-loop** hook: there is always a person at the
prompt. The unattended scenario `ci` was built for does not occur in this deployment,
so the level would never be selected — dead weight in the ladder. Its one durable idea,
*prefer pinned/verified provenance*, is a **preference knob**, not a capability tier: a
user on a sensitive repo may want "`developer`, but require hash-verified installs."
That is a **modifier** dialed on top of a level — exactly the fate of containment above.
So `ci` is **retired as a level**; the provenance-strictness becomes an optional
`pinned-provenance` modifier (off by default) that tightens the supply-chain clause of
whatever level is active:

```toml
[modifier.pinned-provenance]           # opt-in; tightens supply-chain on the active level
supply_chain = { pinning = ">= hash-verified", source ≠ "unverified-url" }
```

Like every modifier it transforms the profile-check, not the level, so it composes with
`developer`, `admin`, or `infra` for free — and, being off by default, costs nothing to
the everyday user who never runs unattended.

**R27 still stands — `extends` composes *upward* only.** With `ci` gone, every
remaining level is either looser (built by `extends`) or a deny-by-default sibling
(`admin`/`infra`) authored up from a low base. No shipped level is
"`developer`-minus-something," so the extend-only-loosens rule is never fought. The
open question R27 raised — *does the level language need a `restricts` primitive to
author **stricter** levels?* — is answered **no**: a stricter level is always built up
from a lower base. §6 shows the one place a subtractive primitive *does* earn its keep,
and it is the opposite case — the **loosest** level, removing a few catastrophe corners
from allow-almost-everything.

**The dissolved conflation, cleanly:** the old `ci`/`contained-mode` was two orthogonal
axes, and **both** turn out to be modifiers, not levels — **containment** (isolation,
§3.2) and **pinned-provenance** (this section). Neither is a tier; both transform the
profile the active level judges. That is the third false level the model has shed
(recursion axis → gone; containment → modifier; `ci` → modifier), and each removal made
the ladder simpler and more honest.

---

## 6. The `yolo` level — allow-almost-everything, minus catastrophe (opt-in)

**Intent:** "I am on a machine I own or can throw away — a personal dev box, a VM, a
container I will delete. Stop asking me about `sudo`, `rm`, and installs. Still stop me
from the handful of things that can't be undone." It is the top of the *local* ladder,
strictly looser than `developer`, and **off by default** — opted into per-environment
through the same trusted-config gate `admin`/`infra` use. It folds in the
non-catastrophic parts of `admin` (local root: `sudo apt install`, `systemctl restart`,
editing `/etc`) so privileged-local friction disappears, and it loosens `developer`'s
network clause to allow arbitrary-destination **fetches** (pulling data can't wreck a
box). It draws the line at five **catastrophe corners** — irrecoverable acts it refuses
even here:

- **C1 — irrecoverable wide destruction.** `destroy ∧ reversibility = irreversible ∧
  (scale ≥ machine-wide ∨ target ∈ {block-device, filesystem, system-path})`. `mkfs`,
  `dd of=/dev/sda`, `rm -rf /`, `rm -rf ~`, repartition. Project-scoped deletes
  (`rm -rf ./node_modules`, `rm -rf ~/oldproject`) stay allowed — bounded, recoverable.
- **C2 — kernel / firmware / device state.** raw block-device writes, `insmod`/`modprobe`,
  firmware/EFI/NVRAM flash. This is the `device`/`kernel` line `admin` already draws;
  `yolo` keeps it. (Editing `/etc` — a reversible `machine`-locus mutate — stays allowed.)
- **C3 — unverified remote code as root.** `execute ∧ authority = root ∧
  supply_chain.source = unverified-url` — the `curl … | sudo bash` shape. `yolo`
  licenses *your* commands, not the internet's, at root. (`curl … | bash` as the *user*
  is allowed — recoverable, userspace.)
- **C4 — anything that leaves this machine.** remote *mutation* (`locus.remote ≥
  mutate`: `git push`, `kubectl apply`, cloud-resource writes) and outbound sends
  carrying host data. `yolo` is a **local** license (the SafeWrite-scope principle); it
  does not extend trust to the network. Remote fetch (`git fetch`, `npm install`) stays
  allowed; remote mutation remains `infra`.
- **C5 — secret disclosure to chat / external.** `secret = true ∧ disclosure.audience ∈
  {chat, external}` (HP-15). A leaked key never un-leaks. Piping a secret into a local
  tool that consumes it is fine; dumping it where it persists is not.

Everything *not* in C1–C5 auto-runs: arbitrary local code, any local edit, installs
(pinned or floating), local `git`, and privileged local administration short of the
corners.

### 6.1 Why `yolo` needs a subtractive primitive (and stricter levels don't)

`yolo` is the first level whose natural definition is "allow almost everything **except**
a few corners." Every other level is a union of positive allow-boxes — you state what is
*in*. `yolo` is the complement: a maximal-local allow with holes punched in it. A
positive-only language can only express a hole by *tiling its complement* — to exclude
one 3-facet corner `{destroy ∧ irreversible ∧ wide}` you write three overlapping boxes
whose union is everything-but-the-corner. For one corner that is tolerable; for the
genuinely-interior corners here (C1 and C3 sit *inside* the maximal allow's box) it
explodes into an unauditable pile of clauses. (C2/C4/C5 fall out of *scoping* the
positive allow — simply don't grant device/kernel, remote-mutation, or host-data sends —
so only C1 and C3 truly need subtraction.)

So `yolo` is the honest motivation for a **bounded, allow-only** subtractive clause:

```toml
[level.yolo]                           # opt-in; top of the local ladder
[[level.yolo.allow]]                   # maximal LOCAL grant: any op, up to root, any scale
operation     = ["observe","create","mutate","destroy","execute","communicate","configure","control"]
locus         = { local = "<= machine", remote = "<= fetch-only" }   # machine, not device/kernel (C2 by scope)
authority     = "<= root"
persistence   = "<= installing"
scale         = "<= unbounded"
reversibility = "<= irreversible"
execution     = "<= network-sourced"
network       = { direction = "<= outbound", destination = "<= arbitrary", payload = "<= fetches" }  # C4/C5 by scope
[[level.yolo.deny]]                     # C1 — irrecoverable wide destruction (interior corner)
operation = ["destroy"]; reversibility = "irreversible"; scale = ">= machine-wide"
[[level.yolo.deny]]                     # C1b — irreversible write over a system path / whole fs
operation = ["destroy","mutate"]; reversibility = "irreversible"; locus = { local = ">= machine" }
[[level.yolo.deny]]                     # C3 — unverified remote code as root (interior corner)
operation = ["execute"]; authority = "root"; supply_chain = { source = ["unverified-url"] }
```

`deny` clauses are evaluated **after** allows and only ever *remove* capability — a
`deny` can never grant. That is what makes the primitive safe to add: it is
**monotonic-downward**, so it cannot be misused to sneak capability into a level (the
R27 worry runs the other way — it feared a subtractive form used to *forge* a stricter
level from a looser base). This resolves R27's open question cleanly: the level language
wants **not** a `restricts`-for-stricter primitive, but a `deny`-for-the-loosest one,
used solely where the honest shape is "allow-all minus catastrophe." `yolo` is its only
client.

### 6.2 Contract and tests
- **`yolo ⊃ developer`** and **`yolo ⊃ (admin ∩ ¬catastrophe)`**; `infra` (remote
  mutation) stays outside it by C4. `yolo` is the top of the local ladder.
- **Proptest additions:** (a) *deny-monotonicity* — adding any `deny` clause only shrinks
  the admitted set (∀ profile: `admits(yolo) ⇒ admits(allow-only-yolo)`); (b)
  *catastrophe-floor* — every profile matching C1–C5 is denied by `yolo` (the golden-set
  `mkfs` / `dd of=/dev/sda` / `curl|sudo bash` / `git push` / secret-to-chat rows all
  `✗`); (c) *no-forge* — the deny-only property: `yolo` never admits a profile its
  maximal allow didn't already grant.
- **Golden-set** gains a `yolo` column right of `developer`: `✓` on `sudo apt install`,
  `sudo systemctl restart`, `rm -rf ~/oldproject`; `✗` (ask) on `git push`,
  `dd of=/dev/disk2`, `mkfs`, exfil, cloud writes.

---

## Net effect

**New findings for v1.3:**
- **R24** — Persistence trigger is `escape ∈ {immediate < detached < recurring <
  boot}` (the ordinal levels gate) + `kind ∈ {clock, event}` (categorical, under
  `recurring`). v1.2's flat five-term ordinal was wrong.
- **R25** — Locus splits into `local` (ordinal, `process…kernel`) + `remote`
  (separate, carries destination/pinned-vs-ambient). v1.2's single ordinal
  mis-ordered `kernel` vs `remote`.
- **R26** — `infra` is remote-cloud-operator; `admin` (local-privileged) is a
  distinct sibling. `infra` operationalizes HP-12 (`remote = pinned`) and gates
  irreversible remote destroy to a prompt.
- **R27** — `extends` composes *upward* (unions allow-clauses → looser). A stricter
  level must be authored from a lower base plus a tightened clause, not by restricting a
  looser one. Extend to loosen; build up to tighten (§5). With `ci` retired, no shipped
  level fights this rule; a subtractive primitive is needed only for the *loosest* level
  (§6), never a stricter one.
- **§4/§5 resolutions** — `admin` fully specified (local root, deny-by-default,
  four exclusions). Both halves of the old `ci`/`contained-mode` conflation resolve to
  **modifiers, not levels**: containment → the isolation modifier (subsumed by §3.2,
  HP-2), and unattended-provenance → the optional `pinned-provenance` modifier (HP-1).
  `ci` is retired as a level.

**Level ladders pinned (across the shipped set):**
- trigger: immediate (strict) → detached (developer) → boot (infra).
- locus.local: temp/user/worktree (strict) → worktree-trusted (developer) → machine
  (admin); `device`/`kernel` never; `remote`: none → fetch-only (developer) →
  mutate-pinned (infra).

**Folds into:** trigger + loci → the facet sections of a v1.3 spec; `infra` (and the
`admin` sibling) → `behavioral-taxonomy-levels`. HP-12 gains a concrete mitigation
(`locus.remote = pinned`) worth noting in the log.

**Revised default set:** `inert ⊂ read-local ⊂ write-local ⊂ developer ⊂ yolo`, with two
deny-by-default siblings off `developer` — **`admin`** (local root) and **`infra`**
(remote cloud) — plus two **modifiers** that apply to any level: **containment**
(isolation, §3.2) and **`pinned-provenance`** (opt-in supply-chain tightening). `ci`
and `contained-mode` are both gone as levels. **`yolo`** (§6) is the opt-in top of the
local ladder — allow-almost-everything minus five catastrophe corners — and the sole
client of the level language's bounded, allow-only `deny` clause.

**Still open:** the exact effect (if any) of the `recurring` trigger *kind*
(clock vs event) on admissibility; and the per-ecosystem definition of `pinning ≥
version` / `≥ hash-verified` (the supply-chain sub-facet catalog, annex `delegation`).
The R27 `restricts`-primitive question is resolved: not needed for stricter levels; a
bounded subtractive form is used only by the loose level (§6).
