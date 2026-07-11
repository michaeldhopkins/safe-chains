# Sub-command archetypes — the Phase 1 classification reference

Status: draft (2026-07-15). Companion to `behavioral-taxonomy-toml-bridge.md` (Phase 0) and
`behavioral-taxonomy-levels.md` (the level model). Phase 1 classifies the ~3,600 `candidate = true`
subcommands across ~256 files by DECLARED facets instead of hand-marking. This doc is the reference:
the recurring facet profiles ("archetypes"), each an audited bundle a sub can reference, plus the
finding that the facet vocabulary is complete for this surface.

## 1. The finding

A 16-family survey — PaaS (platform, scalingo, koyeb, railway, clever), cloud/orchestration (oc,
amplify), object storage (mc, ipfs), secrets (vault), blockchain (solana, spl-token), VCS (fossil,
svn, hg), package/build (spack, nix), containers (nerdctl) — shows the candidate surface is far more
regular than its size suggests. Almost every tool's own description splits into **read-side** and
**write-side** subs, and the write-side separates along a *small, fixed* set of facets:

- **locus** — does the effect land locally or on a REMOTE system? (the dominant discriminator)
- **operation** — observe / mutate / create / destroy / authorize / execute / control
- **reversibility** — recoverable-with-effort vs irreversible (blockchain, data deletes)
- **network payload** — a fetch, or does it SEND host data (deploy/push/upload)?
- **execution** — does it fetch and RUN code (installs, builds)? → supply-chain
- **cost** — does it provision metered/quota resources?

Every sub in the survey maps to one of the archetypes below using only these axes. **No sub required
a facet the vocabulary lacks** — the completeness check for the candidate surface passes. (Where a
future sub resists every archetype, that is the signal to extend the vocabulary, fail-closed, not to
force a fit — same rule as the `unclassified` term in the provenance schema.)

## 2. The archetype catalog

Each archetype is a fully-explicit facet bundle (the columns are the facets that define it; unlisted
facets are at their zero term). The **Lands at** column is derived by running the bundle through the
authored level algebra (`levels/default.toml`) — not asserted — so these are consistent with the
levels as built.

| Archetype | operation | locus | network | reversibility | other | Lands at | Examples |
|---|---|---|---|---|---|---|---|
| **remote-read** | observe | remote=fixed | outbound·**fetch** | none | disclosure→local-process | **reader** | `koyeb list`, `oc get`, `mc ls`, `vault read`, `railway status`, `scalingo apps` |
| **data-export** | observe | remote=fixed | outbound·**fetch** | none | **scale=unbounded**, disclosure→local-process | **reader** | `supabase db dump`, `pg_dump`, `mysqldump`, `<cloud> db dump/export` |
| **bulk-object-read** | observe | remote=fixed | outbound·**fetch** | none | scale=unbounded, **secret=reads**, disclosure→local-process | **yolo** | `aws s3api get-object`, `glacier get-job-output`, `ebs get-snapshot-block`, `omics get-read-set` |
| **remote-mutate** | mutate/configure | remote=fixed | outbound·sends-host-data | ≤ effortful | — | network-admin | `oc apply`, `vault kv put`, `mc cp/mirror`, `railway up`, `koyeb deploy`, `svn commit` |
| **remote-create** | create | remote=fixed | outbound·sends-host-data | ≤ effortful | cost=metered | network-admin | `railway init`, `amplify add`, `mc mb`, `koyeb service create` |
| **remote-destroy** (recoverable) | destroy | remote=fixed | outbound | ≤ effortful | — | network-admin | `oc delete pod`, `railway down`, `koyeb delete`, `amplify remove` |
| **remote-destroy** (irreversible) | destroy | remote=fixed | outbound | irreversible | scale=bounded | **yolo** | `mc rb --force`, `vault kv delete` (no versioning), `railway delete` |
| **remote-authorize** | authorize | remote=fixed | outbound | ≤ effortful | secret=writes/transmits | network-admin | `vault token create`, `vault login`, `spl-token approve/revoke`, PaaS `login` |
| **remote-control** | control | remote=fixed | outbound | ≤ effortful | — | network-admin | `vault seal/unseal`, `railway restart`, `oc rollout` |
| **vcs-sync** | communicate | remote=fixed | outbound·sends-host-data | ≤ effortful | — | network-admin | `fossil push/pull/sync`, `hg push/pull`, `git push`, `svn update` |
| **supply-chain-build** | execute/create | local≤worktree-trusted | outbound·fetch | ≤ effortful | execution=network-sourced, persistence=installing | developer (pinned) / above | `spack install`, `nix build/run`, `amplify push` (codegen), package installs |
| **blockchain-txn** | create/mutate | remote=fixed | outbound·sends-host-data | **irreversible** | cost=metered, secret=uses-ambient | **yolo** | `solana transfer/deploy`, `spl-token mint/burn/transfer` |
| **local-vcs-write** *(deferred, §5)* | create | local | none | ≤ recoverable | — | editor | `hg commit`, `fossil add`, `git commit` (DVCS local) |
| **local-privileged** | configure/control | local=machine | none | ≤ effortful | authority=root | local-admin | `sudo systemctl`, package installs to /usr, mounts |

Two structural notes:

- **Assignment is per-SUB, not per-command.** A DVCS mixes archetypes: `hg commit` is *local-vcs-write*,
  `hg push` is *vcs-sync*, `hg incoming` is *remote-read*. A PaaS mixes *remote-read* (`list`) with
  *remote-mutate* (`deploy`) and *remote-destroy* (`delete`). So the archetype is a property of the
  subcommand, and the read/write split in each tool's description is (usually) the archetype boundary.
- **Reads and writes split at the network boundary.** A pure remote *fetch* (`remote-read`) is a
  **reader**-level read and AUTO-APPROVES — a read is a read regardless of where the data lives; only
  `paranoid` blocks the network, and the "attacker-controlled response → model" risk is prompt-injection
  (the model's/harness's job, out of scope). The dangerous compositions your intuition worries about are
  caught elsewhere, fail-closed on the *primitive*: `curl … | sh` denies because `sh` executes opaque
  input, `rm $NETVAL` worst-cases an unknown operand, `curl -d @secret` denies on `sends-host-data`, and
  a URL splicing in a secret dies on the inner secret read. So no cross-command taint is needed.
  Remote **writes** (`remote-mutate`/`create`/`destroy`/`authorize`/`control`, `vcs-sync`) are the
  network-admin surface — *that* is where the local-vs-remote nuance and `candidate = true` live. The two
  irreversible write archetypes (remote-destroy-irreversible, blockchain-txn) escape network-admin upward
  to yolo via the reversibility spine.
- **Archetype-tagging is safe on subs that DENY; auto-approving reads keep their flag policy.** A write
  sub tagged `remote-destroy` denies regardless of flags, so the archetype (which classifies the
  operation, not the flag surface) is complete. A `remote-read` sub AUTO-approves, so it should keep its
  standalone/valued flag policy (an archetype alone wouldn't reject an unknown flag) — or a future
  refinement composes archetype + flag validation. So the koyeb pilot tags the write actions and leaves
  the read actions on their existing policy.

- **`data-export` is a bulk read with an optional local SINK.** A db dump (`supabase db dump`,
  `pg_dump`) is a *read* — it auto-approves like `remote-read` — but two nuances distinguish it from a
  point fetch, and both are recorded, not hand-waved. **Volume:** `scale = unbounded` marks that the
  whole dataset flows to the caller; a string classifier can't read the data's *sensitivity*, so it is
  proxied by scale × disclosure (volume × destination = the blast radius), never judged. `scale` does
  not itself gate a read today (a bulk read is still a read, like `cat bigfile`) — it is the honest
  facet, ready if we ever gate remote bulk egress. **The output file:** when the dump writes to `-f
  path` / `--file`, the resolver adds a SECOND capability — a path-gated local `create` at that file's
  locus — so `-f ./out.sql` stays a worktree write (local, auto-approves) while `-f /etc/passwd` gates
  on locus (denied). Declared as `profile = "data-export"` + `output_path_flags = [...]`; the engine
  composes the two caps and the level algebra takes the max. Every spelling of the output flag is
  matched, including the glued short `-fPATH`, so it can't slip the gate.

- **`bulk-object-read` is a POLICY deny the facets don't derive — tier UNSETTLED.** Retrieving an
  arbitrary stored object (`aws s3api get-object`, `glacier get-job-output`, `ebs get-snapshot-block`,
  `omics get-read-set`) is, by every axis we have, a `reader`-level remote fetch-read — the facet model
  auto-approves it, exactly as it would `curl`-ing a URL. But the *content* is opaque bytes the
  classifier cannot assess (a stored `.env`, a private key, a DB dump, PHI), and the equivalent
  `aws s3 cp` is not on the allowlist, so auto-approving it is an inconsistency. We deny it — but the
  ONLY facet lever that denies a read is `secret = reads`, which lands the archetype at **yolo**, the
  same tier as `credential-read`. **This tier is deliberately conservative and is flagged unsettled**
  (safe-chains maintainer, AWS batch 2026-07): not every blob is credential-grade, and a *middle* tier
  (`network-admin` — "elevated remote data egress, not credential-grade") would be more proportionate.
  There is **no facet basis for a middle tier today** — a remote fetch-read is `reader`-level on every
  axis, and only `secret` lifts it, overshooting to yolo. A proportionate tier would require a genuinely
  new axis (a "retrieves bulk CONTENT vs METADATA" distinction). Until then the yolo placement stands as
  the conservative choice; revisit when that axis is designed. This is the one place the AWS sweep found
  a capability the existing vocabulary could not classify without a fudge.

## 3. The schema (built — Phase 1 increments 1–3)

An archetype is a **reusable audited bundle** (`archetypes.toml`), referenced by name — never a unit
of analysis that lets a sub skip its own research (the preset warning, toml-bridge §7). A sub carries
the profile **and its research provenance**, as flat fields:

```toml
[[command.sub]]
name = "delete"
profile = "remote-destroy-recoverable"   # inc. 2: engine resolves this to the archetype's capability
fact    = "Deletes the named app and all its services from the control plane via the HTTPS API."
source  = "https://…/docs/cli — `tool apps delete`"
judgment = "Recoverable, not irreversible: redeployable from source; app-held data is a separate, higher sub."
```

Three layers a future researcher can act on precisely: **`fact`** (what the tool documents — re-check
`source` if upstream moves), **`profile`** (our inference — which archetype it maps to; the engine
uses THIS to derive the verdict), **`judgment`** (our stance where the source doesn't decide, optional).
`fact` + `source` are **required** whenever `profile` is set, enforced by
`registry::every_profiled_sub_has_provenance` (fail-closed — you cannot classify a sub above the line
without citing why). The archetype name is validated against `archetypes.toml`
(`every_sub_profile_names_a_real_archetype`); an unknown name worst-cases at runtime.

This collapses the ~3,600 hand-marked candidate subs into ~10 audited bundles + a two-line
per-sub citation, and makes "adding a cloud CLI" fast: tag each write sub with its archetype + `fact`
+ `source`, and the levels do the rest.

**Per-FLAG escalation (built).** Where a FLAG changes the profile, `[[command.sub.flag]]` declares it:
each present flag ADDS its `classifies` archetype's capability, and the level algebra takes the max —
so a benign base + a destructive flag lands at the flag's tier. `git push` (pilot) is `vcs-sync`;
`git push --force` adds `remote-destroy-irreversible` and escalates network-admin → yolo. Each flag
carries its own `fact` + `source` (a dangerous flag is exactly where the reasoning must be recorded),
and `classifies` names a real archetype or the `unclassified` fail-closed escape.

```toml
[[command.sub]]
name = "push"
profile = "vcs-sync"
fact = "…"; source = "…"
[[command.sub.flag]]
name = "--force"
classifies = "remote-destroy-irreversible"
fact = "Overwrites the remote ref with no merge check, discarding remote-only commits."
source = "https://git-scm.com/docs/git-push — `--force`"
```

**Provenance is validated at BUILD time** (`build::assert_sub_provenance`, reading the TOML): a
profiled sub / escalating flag missing a `fact`+`source`, or naming an unknown archetype, panics at
registry load — fail-closed, so the research is a required part of the tree, never dead metadata.
(Runtime specs carry only what the engine uses: the `profile` and each flag's `name`+`classifies`.)

**Value matching (built).** A flag with `value_prefix` escalates only when its VALUE starts with the
prefix — the space form (`-c core.sshCommand=…`) or glued (`--flag=core.sshCommand=…`) — so ONE valued
flag is benign for most values and dangerous for a specific key. Without `value_prefix` a flag
escalates on mere presence (bare `--force`, or a valued flag like git push `--receive-pack=<program>`
whose *any* value is out of bounds → `unclassified`).

```toml
[[command.sub.flag]]
name = "-c"
value_prefix = "core.sshCommand="   # escalates only for this config key
classifies = "unclassified"          # arbitrary local command → fail-closed worst
fact = "…"; source = "…"
```

**What's NOT covered.** (1) The *canonical* value-pattern case, git's global `-c core.sshCommand=`, is a
COMMAND-level flag (before the subcommand), so the sub-level `value_prefix` mechanism doesn't reach it
— `git -c … push` still falls to the git handler's own `-c` filter. Command-level escalating flags
(`[[command.flag]]`) would let that move to declarative TOML — a follow-on. (2) POSITIONAL value
patterns (`chmod +s`, a `git push +refspec`) — the escalator matches flags, not positionals.

## 5. Open questions

- **`local-vcs-write` and the `.git` locus (deferred).** `.git/` is `worktree-trusted` in the locus
  ladder, so a `git commit` — which writes `.git/objects` and `.git/refs` — is technically a
  worktree-trusted write and lands *above* `editor`, which is wrong (a commit is a routine safe op).
  The real discriminator is **persistence**, not locus: a commit is `persistence = data` (content
  blobs, ref pointers — cannot alter future execution), whereas `git config` is `reconfiguring` and
  installing a `.git/hooks/` script is `installing`. So the honest fix is to let `editor` admit a
  worktree-trusted write **when `persistence <= data`**, catching the hook/config writes by their
  persistence rather than excluding commits by their locus — with the resolver responsible for
  scoring a `.github/workflows` or `.git/config` write as `reconfiguring`/`installing`, not `data`.
  This is a leaf-command (git/hg) concern, not the remote candidate surface, so it waits.

## 4. Next

Build the schema: the orthogonal-facet fields on `[[command.sub]]` (the archetype bundles as explicit
facet declarations), the `profile = "…"` expansion, the per-item provenance block, and the proptests
(expansion-matches-catalog; every non-`benign` item has `fact`+`source`; the corpus gate that every
converted `candidate` sub still projects to Denied unless it deliberately lands in an auto-approve
level). Pilot on one file end-to-end — a small PaaS (koyeb: clean read/mutate/create/destroy split) —
behind that corpus gate, then fan out.
