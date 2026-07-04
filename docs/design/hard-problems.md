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

### HP-1 · Contained vs. unattended are two axes, not one — `status: open`
The `ci` level fused two independent ideas: **unattended** (no human to catch a
bad dependency → *tighten* provenance: pinned, no `curl|sh`) and **contained** (a
sandbox bounds blast radius → *relax* reach). They pull in opposite directions and
have no reason to travel together. Renaming `ci` → `contained-mode` names only the
second. The model has no vocabulary for "unattended" as a first-class condition.
*Lead:* maybe two orthogonal knobs (a provenance floor + a containment credit)
rather than a named level. See HP-2.

### HP-2 · Is containment a level or a modifier? — `status: open`
A confirmed sandbox shifts the admissible region of *whatever level you're in*
(isolation clamps locus → `sandbox-scope`, v1.1 §3.2). That is the behavior of a
**modifier** applied to a profile, not a distinct level. If so, `contained-mode`
shouldn't exist as a rung — instead every level has a "…-in-a-sandbox" variant
derived by the same transform. Open question: does the modifier compose cleanly
with the level predicate (transform the profile, then test), or does the
*isolation credit* (does this level trust this sandbox kind?) entangle the two?
*Lead:* model modifiers (isolation, `sudo`/privilege, `--dry-run`) as profile
transforms evaluated before the level predicate; keep the "credit" decision on the
level side.

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
*on the command line* (`--context`, `--profile`, explicit host).

---

## Parked policy decisions

Not modeling gaps — real choices deferred until the data model is better
understood (`behavioral-taxonomy-levels.md` §6):

- **Floating versions in `developer`** — deny unpinned `pip install foo` /
  `npm i foo@latest` (a build script from an unaudited resolved version runs)?
- **Exec-surface ceiling** — is `install-hook` (npm preinstall) inside or outside
  `developer`, or only `build-script`?
- **Bounded destroy in `write-local`** — does `rm ./file` (non-recursive, in-tree)
  belong in `write-local`, or only `developer`?
- **Per-ecosystem `pinning ≥ version` test** — what counts as "pinned" for each of
  npm / pip / cargo / go / apt.

---

## Solved → promoted (keep the trail)
- **Delegation recursion depth** → not a real problem; compounding is chain
  segmentation + the Scale facet, not a recursion bound (v1.1 §3.1).
- **Level definitions look arbitrary** → the TOML clause model + facet-monotonicity
  proptest makes "never arbitrary" enforceable (v1.1 §4.1–4.2).
