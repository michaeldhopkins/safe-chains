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

**Unattended → a level (`ci`).** This axis *does* change the predicate: with no human
watching, the acceptable supply-chain is tighter. So it is a real level — `ci` — a
**stricter** `developer`:

```toml
[level.ci]                             # unattended pipeline: stricter provenance
extends = "write-local"                # NB: built up from write-local, not down from developer (R27)
[[level.ci.allow]]                     # builds, but only hash-verified from signed sources
operation    = ["execute"]
execution    = "<= network-sourced"
supply_chain = { source = ["signed-repo", "private-registry", "vendored"],
                 pinning = ">= hash-verified", exec_surface = "<= build-script" }
[[level.ci.allow]]                     # outbound fetch to fixed endpoints
operation = ["communicate"]
network   = { direction = "<= outbound", destination = "<= fixed", payload = "<= fetches" }
```

`ci` forbids the floating-tag installs and public-registry / unverified-url bootstraps
that `developer` tolerates, because an unattended run can't notice a swapped
dependency. It is a level a pipeline *selects*; being stricter, it needs no
deny-by-default gate.

**R27 — `extends` composes *upward* only.** Note `ci` is built from `write-local`
plus a tight build clause, **not** by "extending `developer` and restricting it."
The level-TOML `extends` unions allow-clauses, which only ever makes a level *looser*;
inheriting `developer` would drag in its permissive build clause and a floating-tag
install would match it. So a **stricter** variant must be authored from a lower base
plus its tightened clause. `admin`/`infra` (supersets of `developer`) extend it
correctly; `ci` (a subset) cannot. Authoring discipline: extend to loosen, build up
from a lower base to tighten.

**The dissolved conflation, cleanly:** the old `ci`/`contained-mode` splits into the
**isolation modifier** (contained; §3.2, no level) and the **`ci` level** (unattended;
stricter build), which *compose* — a containerized CI job is the `ci` predicate over a
sandbox-clamped profile.

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
  level (`ci`) must be authored from a lower base plus a tightened clause, not by
  restricting a looser one. Extend to loosen; build up to tighten (§5).
- **§4/§5 resolutions** — `admin` fully specified (local root, deny-by-default,
  four exclusions). `contained-mode` retired as a level: containment is a **modifier**
  (subsumed by §3.2 isolation, HP-2), and the unattended axis becomes the **`ci`**
  level (stricter `developer`, HP-1).

**Level ladders pinned (across the shipped set):**
- trigger: immediate (strict) → detached (developer) → boot (infra).
- locus.local: temp/user/worktree (strict) → worktree-trusted (developer) → machine
  (admin); `device`/`kernel` never; `remote`: none → fetch-only (developer) →
  mutate-pinned (infra).

**Folds into:** trigger + loci → the facet sections of a v1.3 spec; `infra` (and the
`admin` sibling) → `behavioral-taxonomy-levels`. HP-12 gains a concrete mitigation
(`locus.remote = pinned`) worth noting in the log.

**Revised default set:** `inert ⊂ read-local ⊂ write-local ⊂ developer`, with three
siblings off `developer` — **`ci`** (stricter provenance; a pipeline selects it),
**`admin`** (local root; deny-by-default), **`infra`** (remote cloud; deny-by-default)
— plus the **isolation modifier** (containment) that applies to any level via §3.2.
`contained-mode` is gone.

**Still open:** the exact effect (if any) of the `recurring` trigger *kind*
(clock vs event) on admissibility; whether `ci`'s tightened source set should be
per-ecosystem; and — surfaced by R27 — whether the level language wants an explicit
`restricts`/override primitive, or whether "author stricter from a lower base" is a
sufficient discipline.
