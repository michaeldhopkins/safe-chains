# Behavioral Capability Model — spec v1.3 (canonical)

Status: canonical draft (2026-07-04). Supersedes `behavioral-taxonomy-v1.2.md`.
A touch-up folding the three refinements (R24–R26, annex
`behavioral-taxonomy-refinements`) into the facet and level definitions; it
otherwise carries the v1.2 model whole (three pilots, 65 commands, R1–R23; the
payload frame + resolution depth; the survey; HP-1–14). Not yet implemented.

**Changes from v1.2** (corrections, integrated):
- **Locus splits into two axes** (R25): a `local` depth ordinal
  (`process … machine < device < kernel`) and a separate `remote` reach carrying
  destination + pinned-vs-ambient. `kernel` and `remote` are not comparable, so a
  single ordinal mis-ordered them.
- **Persistence trigger is not a flat ordinal** (R24): an `escape` ordinal
  (`immediate < detached < recurring < boot`) plus a `kind` (`clock`/`event`,
  categorical, under `recurring`). v1.2's five-term line ranked `scheduled`/`event`
  falsely.
- **`infra` is remote-cloud-operator** (R26); local-privileged **`admin`** is a
  distinct sibling. `infra` operationalizes HP-12 (`locus.remote = pinned`) and gates
  irreversible remote destroy to a prompt. `device`/`kernel` are deny-by-default at
  every shipped level.

(For the v1.1→v1.2 changes — payload frame, resolution depth, device/kernel loci,
open-set channels, injection-point modifiers, leaf-form granularity — see v1.2's
header; all carried forward here.)

---

## 1. The model

- **Command-form** — a specific invocation shape: base command + subcommand path +
  flags present + positional-argument shapes.
- **Capability** — one thing a form can do: an *operation* plus values on the
  applicable *facets* (§2).
- **Behavior profile** — the *set* of capabilities a form exhibits.
- **Facet** — one orthogonal dimension, with a small operationally-defined
  vocabulary. Ordinal facets carry a severity order; a few are compound.
- **Safety level** — a named **admissibility predicate** over facet-space (§4),
  expressed as TOML data. A profile passes iff every capability is admissible **and**
  the flow policy holds.

```
invocation ──parse & resolve──▶ behavior profile (set of capabilities)
             safety level L (a predicate) ──▶ admit iff every capability ∈ L
                                              AND the flow policy holds
                                        ▼   allow · prompt · deny
```

**Granularity (R8).** The conversion unit is `(tool, subcommand-path, flags,
arg-shapes)` — the *leaf form*, never the binary. A deep-tree tool (`git`, `aws`,
`docker`, `kubectl`) is a **forest of profiles** spanning inert→irreversible-remote;
its TOML must express many leaf profiles cheaply.

**Allowlist-only invariant.** A form with no profile is unclassified and never
auto-approved. Any unresolved delegation, argument, facet, or resolution depth
resolves to its worst term. The taxonomy only enriches the *approved* side.

---

## 2. The taxonomy — facets

Six groups. Ordinal terms low→high severity; each carries a **discriminator** (the
test vs its neighbors). Exhaustive examples live in the golden-set (§5) and pilots.

### 2.1 The act
**Operation** (categorical, one per capability): `observe` · `create` · `mutate` ·
`destroy` · `execute` · `communicate` · `configure` (change settings that alter
future commands) · `authorize` (change credentials/trust/access) · `control`
(start/stop/signal processes, services, devices).

### 2.2 Reach
**Locus** (compound — two axes, R25). A single ordinal cannot hold both "how deep
into this host" and "which other host"; `kernel` and `remote` are different places,
not more/less of one thing.
- **`local`** (ordinal depth): `process` → `temp` → `sandbox-scope` (§3.2) →
  `worktree` → `worktree-trusted` (`.git/`, `.envrc`, hooks, CI configs) → `user`
  (`~`, keychain) → `machine` (`/etc`, services, other users) → **`device`** (raw
  block/char devices, beneath the filesystem: `dd of=/dev/rdisk0`, `mount`) →
  **`kernel`** (module/extension load: `kmutil load`). `device`/`kernel` (R18) void
  the abstractions every higher fs rung assumes and are deny-by-default everywhere
  (§4.3).
- **`remote`** (reach): `none` · `fixed` · `arbitrary`, plus a **pinned-vs-ambient**
  bit — whether the target host is named on the command line (`--context`,
  `--profile`, explicit host) or comes from session state. Same axis as
  Network.destination; the pinned bit is what `infra` gates on (HP-12, §4.3).

A predicate reads e.g. `locus.local ≤ worktree ∧ locus.remote = none`.

**Scale** (ordinal): `single` → `bounded` (a glob/dir/explicit list) → `unbounded`
(recursion / mass op). **Scale modifies destroy *and* disclosure** (R23): a
recursive read that bundles a tree (`tar czf - ~/.ssh`) is a higher-severity
disclosure than a single-file read, exactly as `rm -rf` is a higher-severity
destroy than `rm x`.

**Authority** (ordinal): `user` → `elevated` (`sudo`/`doas`) → `root` →
`other-user` (setuid/run-as). Gates whether `machine`+ effects land; the frame a
privilege delegator applies (§3.1).

**Isolation** (ordinal strength): `none` → `view` (`chroot`) → `namespace` → `userns`
→ `vm` → `ocap`. An isolation frame clamps nested locus to `sandbox-scope`; breach
flags re-add loci (§3.2).

### 2.3 Durability
**Reversibility** (ordinal): `none` (pure observe) → `trivial` (idempotent/undo) →
`recoverable` (VCS/recycle/snapshot) → `effortful` (out-of-band backups only) →
`irreversible`. Environment-dependent cases resolve worst-case (HP-8).

**Persistence** (compound). *Level*: `transient` → `data` → `reconfiguring`
(alters future commands) → `installing` (adds executables/services/hooks).
*Trigger* (R16, R24) — how far execution escapes the check — is itself two parts:
- **`escape`** (ordinal, the part levels gate on): `immediate` (done on return) →
  `detached` (one instance survives the session: `nohup`/`setsid`) → `recurring`
  (re-fires until removed) → `boot` (re-fires and survives reboot: `systemctl
  enable`, `@reboot`, login items).
- **`kind`** (categorical, under `recurring`): `clock` (`cron`, `at`) vs `event`
  (`watchexec`, git hooks, `.envrc` on cd) — for the `because` string, not a severity
  rung, because a per-save `event` can fire more than a monthly `clock`.

Trigger is orthogonal to Level: `nohup sleep 1000 &` is `transient · detached`
(escapes the session, installs nothing); `crontab` is `installing · recurring/clock`.

### 2.4 Information exposure
**Disclosure** (ordinal audience): `none` → `local-process` (stdout → the
agent/model provider) → `local-persistent` (other local users) → `trusted-remote` →
`shared-remote` → `public`.

**Secret** (ordinal): `none` → `uses-ambient` → `reads` → `writes` → `transmits`.

Both facets range over a **channel** and a **principal** (R17/R19, HP-13):
- *Channels* are an **open set** — filesystem, stdout-to-model, network, clipboard
  (`pbcopy`/`pbpaste`), IPC, credential-store (keychain), cross-process
  (`lldb -p`, `/proc/*/mem`). Unknown/covert channels (a `/dev/tcp` redirect, a DNS
  label, another process's argv via `ps`) take the **worst term** — the enumeration
  is not assumed closed.
- *Principal*: a read can cross a principal boundary on the same host (another
  process's memory or argv) with no fs or network touch. `secret=reads` and the
  disclosure audience must account for "belongs to another principal," not just file
  paths.

### 2.5 Channel (network)
**Network** (compound): *Direction* `none · loopback · outbound · inbound-listen` ×
*Destination* `n/a · fixed · arbitrary` × *Payload* `none · fetches ·
sends-host-data`. Network is not only via known binaries: a `> /dev/tcp/host/port`
redirect and a data-carrying DNS label are outbound channels (HP-13); detection
cannot key solely on the base command.

### 2.6 Code provenance
**Execution** (compound). Local trust: `none` → `self` → `caller-inline` → `caller-file`
→ `ambient-config` (Makefile/hooks/`.envrc`/plugins — code not on the command line)
→ `network-sourced`. When `network-sourced`, decompose into **supply-chain**:
*source* (`unverified-url`·`public-registry`·`signed-repo`·`private-registry`·`vendored`)
× *pinning* (`floating`·`version`·`hash-verified`·`digest`) × *exec-surface*
(`none`·`install-hook`·`build-script`·`call-time`·`run-artifact`). See annex
`delegation`.

### 2.7 Resource
**Cost** (ordinal): `none` → `local-resource` → `metered` (billable) → `quota`.
Populated for provisioning tools (`terraform apply`, `aws … create`).

### 2.8 The capability record
```
capability {
  operation · locus{local,remote} · scale · authority · isolation
  reversibility · persistence{level, trigger{escape,kind}}
  disclosure{audience,channel,principal} · secret{level,channel,principal}
  network{direction,destination,payload} · execution(+supply_chain) · cost
  delegate: <nested profile | frame> | none        # §3.1
  because: "<discriminator cited>"
  evidence: { source, url, researched_version }
}
```
Facets not listed default to their zero term.

---

## 3. Mechanisms

### 3.1 Delegation (`frame ∘ nested-profile`)
A delegating capability runs a nested computation. It carries a **frame** and a
nested profile that is *resolved* (classify the inner command) or **opaque**
(worst-case). Seven frames (annexes `delegation`, `isolation`, `payload-frame`):

| frame | commands | transform |
|---|---|---|
| transparent | `bash -c`, `timeout`, `env`, `xargs`, `find -exec` | identity (env-with-`LD_PRELOAD`/`PATH` adds `reconfiguring`) |
| privilege | `sudo`, `doas`, `su -c` | nested `authority := elevated/root` |
| remote | `ssh h CMD`, `docker exec`, `kubectl exec` | nested locus → `remote`; intrinsic outbound |
| isolation | `docker run`, `bwrap`, `wasmtime` | clamp nested locus to `sandbox-scope`, re-add breach loci (§3.2) |
| **payload** | `psql -c`, `kubectl apply -f`, `gh api`, `helm`, `terraform` | nested = a typed document/request in a remote-effect language; decomposed by gates (§3.1.1) |
| task-runner | `make`, `npm run`, `just`, `./gradlew` | nested = project-config code (`ambient-config`); opaque unless a trusted-config root (R13) |
| **interactive** | `ssh` (bare), a REPL, `vim :!`, `docker run -it` | nested = *future input*, unavailable at check → opaque, unbounded (R16) |

Resolution re-enters the classifier on the nested command; frames compose
(`sudo ssh h 'CMD'` = privilege ∘ remote). Nesting is shallow, so resolution runs to
completion; an unparseable inner command is opaque → worst-case. Cross-command
compounding (`a && b | c`) is **not** delegation — per-segment chain evaluation
handles it; fan-out (`xargs`, `find -exec`) is the **Scale** facet.

#### 3.1.1 The payload frame — four gates, a resolution ladder
The payload frame unifies R9 (declarative-config) and R11 (API): both delegate to a
typed payload, already what the interpreter frame named. It decomposes into four
gates that are also increasing **resolution** (§4.4), so a level engages only as
deep as it needs (annexes `payload-frame`, `payload-survey`):
- **Gate 1 · verb** — `method`/`subcommand`/`sql_verb` bounds the operation.
- **Gate 2 · source integrity** — the provenance axis on the *document*: inline >
  in-repo pinned (`worktree-trusted` under a trusted-config root) > URL > stdin.
  Low integrity → worst-case, skip gates 3–4.
- **Gate 3 · selector** — path/kind+namespace/table; a pattern allowlist.
- **Gate 4 · payload predicates** — a nested-grammar allowlist over the body
  (safe-chains recursed one level down).

**Structured-surface rule.** Where the tool exposes verb+resource as *subcommands*
(`aws ec2 run-instances`, `kubectl delete ns/prod`) it is already tree-allowlisted;
engage payload parsing only for the **raw hatch** (`-X`, `-c`, `--raw`, `-f -`). The
survey's build order: k8s R3 (gate 4) first; REST decides at gate 1+2 and a
structured body recurses to its language's resolver; SQL is payload-forbid or a
narrow read-only sublanguage; HCL stops at gate 1 (`plan` already executes).

### 3.2 Isolation strength & breaches
An isolation frame is capability reduction; breaches are re-grants. Containment is
*earned* — clamp nested locus only when the sandbox is confirmed and no breach/unknown
flag is present. Breach catalog in annex `isolation` (`-v HOST:CT`, `--privileged`,
`--pid=host`, `--network=host`, docker-socket mounts).

### 3.3 Argument resolution & worst-case
Several facets are functions of argument values (Locus, Secret, Disclosure audience,
Network destination, Scale). A resolution pass maps argument shapes → facet values
via classifiers (`classify_locus`, `classify_destination`), plus:
- **Operand roles (R10)** — for transfers (`rsync`, `scp`, `aws s3 sync`), classify
  which operand is *source* vs *sink*. Direction flips the dominant facet: upload =
  disclosure/secret-transmit; download = local-write integrity; `--delete` adds a
  `destroy` on the sink.
- **Rule:** any value that can't be pinned (`$VAR`, glob, stdin) takes the facet's
  worst term; the assumption is recorded in `because`.

Two limits the pass **cannot** see, logged as open problems:
- **Content-derived locus (HP-11)** — `tar x`/`unzip` write to paths inside the
  archive (zip-slip); locus depends on consumed data, not the command line.
- **Ambient-state locus (HP-12)** — which remote a payload/API command hits is set
  by session state (kube-context, `AWS_PROFILE`, `$DATABASE_URL`), not the command
  line, so the dominant facet (which remote / prod-vs-dev) is invisible. This feeds
  the `locus.remote` **pinned-vs-ambient** bit (§2.2): a target from session state
  resolves `remote = ambient` → worst-case, and `infra` admits remote mutation only
  when `remote = pinned` (§4.3). A mitigation, not a closure.

### 3.4 Information flow
Safety is often a property of *flows*. The shell's data edges are visible in the CST
(pipes, `$()`, redirects, procsubs, vars), so flow analysis is reachability over
that graph (annexes `information-flow`, `flow-engine`). **The facets are the labels:**
confidentiality source = `secret`; sink = `disclosure`/outbound; integrity source =
execution/supply-chain provenance; sink = `execute`/`configure`.

`declassifier` and `endorser` nodes are **curated allowlists, not categories** (R21):
- *Declassifier* — cryptographic protection only (`gpg -e`, `age`, `openssl enc -k`)
  makes a secret safe to a sink. **Obfuscation is not declassification**: `base64`,
  `gzip`, `xxd` do not declassify — treating them as such is an exfil hole.
- *Endorser* — `gpg --verify`, `sha256 -c`, `cosign verify` raise input integrity,
  licensing an otherwise-forbidden low-integrity→exec flow.

Doctrine: **integrity flows are prevented** (untrusted→exec is intra-line-visible and
complete); **confidentiality flows are detected and elevated** (within a line by the
static pass, across observed-file calls by session taint — a principled reduction,
not a cross-session guarantee).

### 3.5 Modifiers
A modifier transforms a profile before the level predicate: `--dry-run` → all
`observe`; `--force` → `irreversible`; `-R` → `unbounded`; `-o FILE`/redirect →
retarget locus. **Injection-point flags/env (R22)** are execute-adding modifiers —
`git -c`, `GIT_SSH_COMMAND`, `-e`/`--rsh`, `--exec`, `LD_PRELOAD`, curl `-K`: they
turn a benign verb into `execute · ambient-config` from the command line and must be
tagged, or a safe subcommand silently gains code-exec. safe-chains already guards
instances (curl headers, the `node --require` value-flag bug); this names the class.

---

## 4. Safety levels

A level is an **admissible region** — a disjunction of **clauses**, each a
conjunction of per-facet constraints (a ceiling on an ordinal, a set on a
categorical) plus a **flow policy** (forbid `secret`→outbound-send; forbid
low-integrity→exec). A capability is admissible if some clause admits it; a profile
passes iff every capability is admissible and the flow policy holds. This is the
allowlist principle one floor up: safe-chains allowlists command shapes; a level
allowlists *behavior* shapes.

### 4.1 A level is TOML data
Three primitives express every level, conditional payload grants included:
`facet = "<= term"` (ordinal ceiling), `facet = ["a","b"]` (categorical set),
`extends = "other"` (inherit clauses, then add).

```toml
[level.write-local]
operation     = ["observe", "create", "mutate"]
locus         = { local = "<= worktree", remote = "none" }   # R25 two axes
reversibility = "<= recoverable"
persistence   = { level = "<= data", trigger = "<= immediate" }  # R24: no deferred exec
scale         = "<= bounded"
network       = "none"
execution     = "<= caller-inline"
authority     = "user"
flow          = { low_integrity_exec = "forbid", secret_outbound = "forbid" }

[level.developer]
extends = "write-local"
locus       = { local = "<= worktree-trusted", remote = "fixed" }
persistence = { level = "<= installing", trigger = "<= detached" }  # background a dev server
[[level.developer.allow]]            # supply-chain build carve-in
operation    = ["execute"]
execution    = "<= network-sourced"
supply_chain = { source = ["public-registry","signed-repo","private-registry","vendored"],
                 pinning = ">= version", exec_surface = "<= build-script" }
[[level.developer.allow]]            # outbound fetch to fixed endpoints
operation = ["communicate"]
network   = { direction = "<= outbound", destination = "<= fixed", payload = "<= fetches" }
```

`device`/`kernel` appear in no level's `locus.local` ceiling — they are
deny-by-default everywhere and require an explicit hand-authored allowance.

### 4.2 The proptest contract
Ceilings + sets make the engine provable (type-directed generation over `Profile`):
**facet-monotonicity** (an admitted profile stays admitted when any one facet is made
*less* severe — a level that fails this is incoherent by construction) · **`extends`
⇒ superset** · **totality** · **round-trip** · **golden-set anchor**.

### 4.3 The default set
Designed in `behavioral-taxonomy-levels.md` + `…-refinements.md`:
`inert ⊂ read-local ⊂ write-local ⊂ developer`, a **`contained-mode`** sibling
(isolation may instead be a *modifier* — open, HP-2), and two deny-by-default
operator levels (R26): **`infra`** (remote-cloud: `terraform apply`, `kubectl
apply`, `aws … create`) and its distinct sibling **`admin`** (local-privileged:
`sudo apt`, `systemctl`, `/etc`). `infra` admits remote mutation only when
`locus.remote = pinned` (HP-12) and caps `reversibility ≤ effortful` so irreversible
remote destroy (`terraform destroy`, `kubectl delete ns prod`) still prompts. Both
are opt-in and deliberately authored — evidence that the default set does not span
the space and that user-defined levels are load-bearing. The trigger/locus ladders
across the set (`…-refinements` §1–2): trigger `immediate → detached → boot`; locus
`worktree → worktree-trusted → machine`, `device`/`kernel` never. Today's flat
`SafeWrite` allow-set is an **impact baseline**, not a spec to reproduce:
divergences are deliberate (`intended-tightening` / `unacceptable-breakage`), logged,
not regressions.

### 4.4 Resolution depth — a level demands only what it needs
A capability is describable at increasing **resolution** (epistemic, distinct from
severity): coarse (cheap, always available) to fine (expensive, sometimes
impossible). A level's predicate names the resolution it needs; the analyzer resolves
**lazily**, and unavailable resolution (no parser, unpinnable argument) → worst-case
→ deny. This is the guarantee that safe-chains **need not become a universal
interpreter.** For a payload command, three coarse postures cover most levels:
`payload-forbid` (decides at presence), `payload-blind allow` (presence + source),
`payload-aware` (descends to the body grammar; denies if no resolver). Deep grammars
are built incrementally per `(language, level)` — k8s first (§3.1.1).

---

## 5. Non-arbitrariness protocol

*Many* levels, never an arbitrary one — enforced structurally:
1. **Named, never numbered.** A facet term needs a definition, a discriminator, and
   ≥2 examples (one positive, one negative near-miss). No `N3`.
2. **Every classification cites its discriminator** in `because`.
3. **Evidence mandatory**: `source` + `url` + `researched_version`.
4. **Golden-set** — a frozen corpus of forms with expected facet values (seeded by
   the three pilots, 65 forms). Any taxonomy change must keep it classifiable and
   re-review the diffs.
5. **Adding a term requires a demonstrated gap** — a real form the vocabulary can't
   express (the bar the pilots meet: R18 device/kernel came from `dd`/`kmutil`, the
   trigger axis from `cron`/`nohup`, the payload frame from `kubectl apply`/`psql`).

The taxonomy's *shape* recapitulates protection theory (CVSS, POLA, ocap, confused
deputy, Denning IFC — annex `safety-foundations`).

---

## 6. Engine, data, and program

**Profile-resolution engine.** Capabilities attach to grammar nodes (additive /
modifier / gating); the engine unions additive capabilities, applies modifiers in a
pinned order (§3.5), resolves argument-derived facets (§3.3) and delegation (§3.1),
builds the CST dataflow graph, then evaluates the active level's predicate + flow
policy at the resolution it demands (§4.4).

**Capability registry** (its own project). Owns the schema, taxonomy/level/golden-set
data, the delegator/breach/supply-chain/payload catalogs, validation (`because` +
evidence + golden-set green), and a compiler emitting the artifact safe-chains links.
Reusable beyond safe-chains (the CLI-design DB and the book derive from it — annex
`safety-foundations` §5).

**Staged roadmap.**
- Stages 0–2 — **done**: model, three pilots, deep-dives, payload survey, this spec.
- **Stage 3 (in progress)** — level design: pin the default set as TOML clauses;
  measure against today as an impact baseline; grow the golden-set. Contents await a
  better-understood data model (`hard-problems`).
- Stage 4 — engine behind a flag; old-vs-new diff over the corpus.
- Stage 5 — migrate the corpus incrementally (un-migrated → legacy tier).
- Stage 6 — extract the registry; safe-chains consumes the artifact.
- Stage 7 — user-defined levels; session-taint store; the first payload R3 resolver
  (k8s).

---

## 7. Decisions resolved vs deferred

**Resolved (v1.2 + v1.3):** the payload frame unifies interpreter/declarative/API;
resolution depth bounds the interpreter surface; `device`/`kernel` loci; scale
modifies disclosure; curated declassifier/endorser; injection-point modifiers;
operand roles; leaf-form granularity. **v1.3 adds:** locus as `local`+`remote` axes
(R25); the trigger `escape` ordinal + `kind` (R24, fixing the severity ordering);
`infra` as remote-cloud with `locus.remote = pinned` and a prompt-gate on
irreversible destroy (R26).

**Deferred:** exact level contents (`developer` supply-chain thresholds, the
`infra`/`admin` predicates beyond the sketch, the `contained-mode`-vs-modifier
question and whether it composes with `infra`); the local-privileged **`admin`**
predicate; whether the channel set can ever be closed (HP-13); golden-set beyond the
pilot seed; compiled-artifact format; per-language R3 resolvers beyond k8s.

## 8. Open problems

Fourteen live entries in `hard-problems.md`, the ones v1.3 does **not** solve: the
contained/modifier question (HP-1/2), cross-session flow (HP-3), env/toolchain
reinterpretation (HP-4), path-shape≠target and content-derived locus (HP-5/11),
indirection (HP-6), opaque interpreter payloads (HP-7), state-dependent
reversibility/scale (HP-8), read-as-exfil across principals (HP-9), composition
(HP-10), ambient-state target locus (HP-12), channel completeness (HP-13), and
deferred/triggered/interactive execution (HP-14). The spec's honesty is that these
are named and worst-cased, not hidden.

## 9. Reference annexes

`behavioral-taxonomy-levels` (default levels) ·
`behavioral-taxonomy-refinements` (trigger axis, sub-machine loci, infra; R24–R26) ·
`…-pilot` / `…-pilot2` / `…-pilot3`
(65-form golden-set seed, R1–R23) · `…-payload-frame` + `…-payload-survey` (the
payload frame and per-language R3 build order) · `…-delegation` (frame algebra,
supply-chain table) · `…-isolation` (strength ladder, breach catalog) ·
`…-information-flow` + `…-flow-engine` (flow analysis, session taint) · `hard-problems`
(open gaps) · `safety-foundations` (theory) · `reading-list` (bibliography).
