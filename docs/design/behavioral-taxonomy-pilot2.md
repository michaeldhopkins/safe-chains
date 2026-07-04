# Behavioral taxonomy — pilot 2 (tree-heavy conversion)

Status: pilot output (2026-07-03). A second design-time conversion of ~22
commands against `behavioral-taxonomy-v1.1.md`, deliberately weighted toward
tools with **deep subcommand trees** and toward the unmodeled corners the first
pilot only grazed. Findings continue the R-series (R8+) and feed `hard-problems`
and the golden-set. Notation: `{operation · facet=value · …}`; ⚠ marks friction.

These are *modeling* classifications from known behavior; a real TOML conversion
would re-research each tool's current version (project rule). The point here is to
break the model, not to ship the entries.

---

## A. Deep trees & state-dependent reversibility

One tool spans the whole severity range; the profile binds to the **leaf form**,
not the tool. `git` alone:

**1. `git status`** — `{observe · locus=worktree}`. Inert-ish.
**2. `git reset --hard HEAD~3`**
`{destroy · locus=worktree · reversibility=?? · scale=bounded}`.
⚠ Reversibility is **repo-state-dependent**: committed work is reflog-recoverable
(`recoverable`); uncommitted work in the index/worktree is `irreversible`. The
command line can't tell which. Worst-case = `irreversible` (HP-8).
**3. `git rebase -i main`**
`{mutate · locus=worktree · reversibility=recoverable(reflog)}` **plus** an
`execute` capability: the rebase todo can carry `exec <cmd>` lines and it launches
`$EDITOR`. `{execute · execution=caller-inline}` at best, `ambient-config` if the
sequence editor is a script. ⚠ An "edit history" verb is also a code-execution
verb.
**4. `git submodule update --init --recursive`**
`{communicate · network=outbound · destination=arbitrary}` (clones from URLs in
`.gitmodules`, attacker-controlled if the repo is untrusted) · `{create ·
locus=worktree}`. ⚠ Destination is **file-derived**, not argument-derived — the
URLs live in a tracked file (cf. R9).

**5. `systemctl restart nginx`** (contrast `systemctl --user …`)
`{control · locus=machine · authority=elevated · reversibility=recoverable ·
persistence=transient}`. ⚠ `restart`≈recoverable, but `stop`/`disable` leave state
changed (`persistence=reconfiguring` for `disable`). The `--user` variant drops to
`locus=user · authority=user`. Same verb tree, different authority/locus per flag.

> **R8.** The conversion unit is `(tool, subcommand-path, flags, arg-shapes)`, and
> a deep-tree tool is a **forest of profiles** spanning inert→irreversible-remote.
> A tool's TOML must express many leaf profiles cheaply; the profile granularity
> is the form, never the binary.

---

## B. "Apply a declarative file" is delegation to a config payload

**6. `kubectl apply -f manifest.yaml`**
`{execute}` of a **declarative payload** → nested `{create/mutate/destroy ·
locus=remote · scale=unbounded · reversibility=effortful · network=outbound ·
secret=uses-ambient}`. The real behavior is in the YAML, not the command line.
**7. `docker compose up -d`**
Reads `compose.yaml` (`ambient-config`) → `{control · create}` containers,
`{communicate · network}` (pulls images = supply-chain), mounts per file
(locus per `volumes:` — may breach to `machine`), `{communicate ·
inbound-listen}` (published ports). A task-runner *and* an isolation frame *and* a
config payload at once.
**8. `helm install app ./chart`**
`kubectl apply` + templating: `{execute}` of a chart that renders to arbitrary
cluster objects. Nested profile opaque; worst-case remote create/mutate/destroy.
**9. `terraform plan`** (contrast #18 pilot-1 `apply`)
Not pure observe: refreshes remote state and can invoke `external`/`http` data
sources → `{communicate · network=outbound · secret=uses-ambient}` and arbitrary
provider code at plan time. ⚠ Even the "read-only" sibling of a mutating command
is not `inert`.

> **R9.** Add a **declarative-config frame** to delegation (v1.1 §3.1): `apply -f`,
> `compose`, `helm`, `terraform` all delegate to a YAML/HCL payload whose
> operations are unknowable statically (could create *or* destroy anything). Same
> shape as the SQL/ssh interpreter frame — the payload is a nested "language."
> Opaque → worst-case; the file is also `ambient-config` on the integrity axis.

---

## C. Container/build execution & supply-chain trees

**10. `docker build -t app .`**
`{execute · execution=network-sourced}` (Dockerfile `RUN` = arbitrary code; base
image pulled) · `{communicate · outbound}` · `{create · locus=worktree}`. The
Dockerfile is `ambient-config`; the base image is `public-registry`, usually
`floating` (`:latest`).
**11. `brew install wget`**
`{execute · execution=network-sourced}` (formula is Ruby; bottle downloaded) ·
`{create/mutate · locus=user · persistence=installing}` · `{communicate ·
outbound}`. `source=public-registry`, `exec-surface=build-script`.
**12. `apt-get install nginx`** (needs `sudo`)
`{execute · execution=network-sourced · authority=root}` (maintainer `postinst`
runs as root) · `{create · locus=machine · persistence=installing}`. ⚠ The
supply-chain sub-facets compose with **authority=root** — worst combination in the
corpus: unaudited code as root.

> Confirms the supply-chain grid (v1.1 §2.6) and shows it must **compose with
> authority** — `pip install --user` (user) and `apt-get install` (root) share an
> exec-surface but land in different trust models.

---

## D. Transfer direction determines the dominant facet

**13. `aws s3 sync ./ s3://bucket`** vs **`aws s3 sync s3://bucket ./`**
Upload: `{communicate · network=outbound · payload=sends-host-data ·
disclosure=shared-remote}` (a potential exfil of the whole tree). Download:
`{create · locus=worktree}` + `{execute? no}` — an **integrity source** writing
remote bytes locally. Same subcommand, opposite risk, decided by operand order.
**14. `rsync -a --delete ./ host:/srv/`**
Upload + `{destroy · locus=remote · scale=unbounded}` (`--delete` removes remote
files absent locally). `network=outbound · secret=uses-ambient`.
**15. `scp host:/etc/passwd .`**
Download: `{create · locus=worktree}` reading a **remote machine** file — the
remote read audience/secrecy is the remote host's, not ours.

> **R10.** Argument resolution (v1.1 §3.3) must classify **operand roles**
> (source vs sink), not just individual path loci. Transfer direction flips which
> of disclosure (upload) or local-write-integrity (download) dominates; `--delete`
> adds a `destroy` on the sink side. "Which operand is the sink" is the resolvable
> fact that sets the profile.

---

## E. API clients & cloud infra

**16. `gh api -X DELETE /repos/o/r`** (contrast `gh api /user` → observe)
`{execute}` of an **HTTP verb+path**, not a shell string → `{destroy ·
locus=remote · reversibility=irreversible · scale=bounded}`. The verb is the
operation discriminator; the path is the locus/scale.
**17. `aws ec2 run-instances --image-id … --count 10`**
`{create · locus=remote · cost=metered · scale=bounded · persistence=installing ·
reversibility=recoverable(terminate)}`. Real money + quota.
**18. `gcloud compute ssh vm --command 'rm -rf /data'`**
Composition of frames: **API/auth** (provisions access) ∘ **remote** (ssh) ∘ the
nested `{destroy}`. Two delegation hops before the payload.

> **R11.** Add an **API frame** to delegation: a single binary (`gh`, `aws`,
> `gcloud`, `kubectl`) reaches a REST/gRPC control plane. Operation ← HTTP verb;
> locus/scale ← path/resource. Un-parsed (or `-X` arbitrary) → worst-case remote
> mutate/destroy. Like the shell delegation frames but the payload is a request,
> not a command line.
>
> **R12 (levels).** #16–#18 (and pilot-1 terraform, kubectl delete) share a trust
> model `developer` must **not** admit: remote, metered, shared blast radius,
> production-affecting. They need a deliberately-authored **`infra`/`operator`**
> level (deny-by-default, opt-in) — evidence that five levels don't cover the
> space and that user-defined levels (v1.1 §4.3) are load-bearing, not a nicety.

---

## F. Indirection & task-runners

**19. `npm run deploy`** (or `make deploy`)
`{execute}` of a **project-defined script** (`ambient-config`) whose body is
arbitrary and may itself `ssh prod`. Nested profile opaque.
**20. `./gradlew build`**
`{execute · execution=ambient-config}` — a wrapper script *in the worktree* that
downloads and runs a pinned Gradle distribution, then runs build logic. The name
looks like a local file; the behavior is a supply-chain build.

> **R13.** Confirms `hard-problems` HP-6. Resolving indirection means reading
> `package.json`/`build.gradle`/`Makefile` — themselves untrusted `ambient-config`.
> So resolution **cannot grant trust**; it stays opaque→worst-case *unless* the
> project is a trusted-customization root (the v0.205.0 hash-pinned trust model).
> Indirection is where the level model and the trust-config model must meet.

---

## G. Content-derived write locus & fan-out

**21. `tar xzf archive.tar.gz`** (and `unzip pkg.zip -d out`)
`{create/mutate · locus=??}`. ⚠ The write paths come from **inside the archive**,
not the command line (a `../../etc/cron.d/x` entry = zip-slip). Locus is a function
of *data the command reads*, attacker-influenceable, unknowable at check time.
**22. `xargs rm < list`**
`{destroy}` fanned out over **stdin** — target set and scale unknowable
(`scale=unbounded` worst-case). Transparent delegation frame + fan-out via the
Scale facet (v1.1 §3.1).

> **R14 → new hard problem HP-11.** *Content-derived write locus.* Extraction and
> templating tools write to paths embedded in data they consume. This is HP-5's
> cousin (path-shape ≠ resolved target) but driven by file/stdin *content*, not a
> symlink. Worst-case: an untrusted archive can write outside cwd → `locus=machine`.
> Statically unverifiable; mitigations (`--one-top-level`, `tar --keep-old-files`,
> `unzip -d` + path checks) are per-tool and not machine-checkable from the shape.

---

## H. Trust-granting & secret-reading

**23. `direnv allow`**
`{authorize · locus=worktree-trusted · persistence=reconfiguring}`. Does nothing
harmful *now*; it marks `.envrc` trusted so arbitrary code auto-runs on every
future `cd` into the dir. A meta-capability: it grants future execution.
**24. `env`** (bare) / `printenv`
`{observe · disclosure=local-process · secret=reads}` — dumps every env var,
including tokens, to stdout → the model. A "read" that is an exfiltration.

> **R15.** `authorize`-operation commands (`direnv allow`, adding an ssh key, `git
> config` of an exec hook) carry no present damage; their entire risk is
> *downstream* (HP-4). They can only be rated against the future capability they
> unlock, which needs the session/flow model to connect grant → later use.
> Confirms HP-9 for `env`: the model/provider is an untrusted disclosure audience
> the level must rate explicitly.

---

## Net effect

**Spec revisions proposed (for folding into a v1.2, like R1–R7 → v1.1):**
- **R9 declarative-config frame** and **R11 API frame** — two new delegation frame
  kinds in v1.1 §3.1; both are "delegation to a non-shell payload," generalizing
  the interpreter-frame note.
- **R10 operand-role resolution** — argument resolution (§3.3) classifies source
  vs sink operands for transfer commands.
- **R12 an `infra`/`operator` level** — a deny-by-default trust model above
  `developer`; feed to `behavioral-taxonomy-levels` §6.
- **R8** — restate the profile-granularity rule (leaf form, forest per tool) in
  §1/§6.

**Hard-problems log:**
- **New HP-11** — content-derived write locus (tar/unzip/templating).
- **Confirmed/strengthened** — HP-4 (env mutation; direnv allow), HP-6 (indirection;
  and its collision with the trust-config model), HP-8 (state-dependent
  reversibility; git reset), HP-9 (read-as-exfil; env).

**Golden-set:** add these 24 forms with per-level expected verdicts. Most land
above `developer` (declarative-apply, infra, root installs), which is the useful
signal — the honest model refuses far more than the flat `SafeWrite` ceiling did,
and each refusal names a fact.
