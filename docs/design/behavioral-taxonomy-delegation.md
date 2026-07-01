# Delegation & supply-chain provenance — deep-dive (spec v1.1)

Status: draft (2026-07-01). Deepens R1 (delegation) and refines F6 (execution
provenance) from the Stage-2 pilot. Feeds back into `behavioral-taxonomy-v1.md`.

Two subjects, one theme: a command's real behavior often lives in code it *runs
on your behalf* — a nested command, a remote shell, a downloaded package. Both
are modeled as a **frame** wrapping a **nested computation**.

---

# Part A — Delegation

## A.1 Definition

A **delegating capability** runs a nested computation whose behavior is (part of)
the command's behavior. It is modeled as:

```
delegate {
  frame:  <how the nested computation is contextualized>
  nested: <profile>  |  opaque      # the nested computation's own behavior
}
```

The delegator's profile = its **intrinsic** capabilities (reaching the host,
changing authority, unpacking an image) **∪** (`nested` transformed by `frame`).

`nested` is either **resolved** — the nested command is a literal we can parse
and classify — or **opaque** — not statically knowable, so it resolves to the
`unclassified` worst-case profile (fails all but the most permissive levels).
This is the safe default and is what makes "safe wrapper + unknown payload"
(`timeout 60 $CMD`, `ssh host "$CMD"`, `xargs`) correctly non-auto-approvable.

## A.2 Delegator catalog (the frames)

Six frame kinds. Each has a defined transform (A.3). The catalog is the
"research": which real commands are delegators and which frame they apply.

| frame | commands (non-exhaustive) | what it does |
|---|---|---|
| **transparent** | `bash -c` `sh -c` `env` `timeout` `nice` `nohup` `stdbuf` `setsid` `time` `xargs` `find -exec` `watch` `parallel` | runs nested in the same host/user context |
| **privilege** | `sudo` `doas` `su -c` `runuser` `pkexec` | runs nested with elevated authority |
| **remote** | `ssh host CMD` `docker exec` `kubectl exec` `nsenter` `mosh` | runs nested on another host/container |
| **isolation** | `docker run IMG CMD` `podman run` `firejail` `bwrap` `chroot` `systemd-run` `nsjail` `nix-shell --run` | runs nested inside a sandbox |
| **interpreter** | `python -c` `node -e` `ruby -e` `perl -e` `awk` `jq` `psql -c` `sqlite3 db SQL` `sed -e` | runs nested code in another *language* |
| **task-runner** | `make TGT` `just` `npm run` `rake` `cargo run` `mise run` `task` | runs nested code defined in *project config* (ambient) |

Notes that matter:
- `find -exec CMD {} \;` — the command *template* is resolvable; the `{}`
  substitutions are per-file arguments. Classify the template with the matched
  paths as `bounded`/`unbounded` scale.
- `xargs CMD` — the command is known but its *arguments* come from stdin →
  arguments opaque; classify the command with worst-case argument facets.
- task-runners are the intersection with **F6 `ambient-config`**: the recipe is
  project-controlled code not on the command line. `make deploy` is opaque unless
  we parse the Makefile (we don't) → worst-case. This is the same self-escalation
  surface as `.envrc`/hooks.

## A.3 The frame algebra (transforms)

Each frame transforms the nested profile. Transforms compose (`sudo ssh host CMD`
= privilege ∘ remote).

- **transparent** — identity on the nested profile. Intrinsic: usually none.
  Caveat: `env LD_PRELOAD=…`/`PATH=…` and `env -i` are `configure` capabilities
  that can change what the nested command resolves to → treat env-with-exec-
  influencing-vars as a `reconfiguring` intrinsic, and the nested command as
  potentially-substituted (worst-case its identity).
- **privilege** — `authority := elevated|root` on every nested capability.
  Effect: nested `user`/`machine` loci that were authority-gated now *land*, and
  `destroy` reversibility escalates (root can remove anything, defeats
  permission-based recovery). Intrinsic: reads sudo config/timestamp.
- **remote** — `locus := remote` for every nested `worktree|user|machine`
  capability (the effect lands on the other host, not here). Intrinsic: network =
  `outbound`, destination = `fixed` if the host is a literal else `arbitrary`;
  `secret=uses-ambient` (keys/agent). Disclosure/reversibility of nested effects
  are evaluated *relative to the remote*.
- **isolation** — `locus` of nested capabilities is **downgraded** toward
  `process`/container-scope (contained), THEN **breach flags re-add loci**:
  - `-v HOST:CT` / `--mount` → re-add a capability at `classify_locus(HOST)`
  - `--privileged` / `--pid=host` / `--network=host` / `--cap-add` / `--device` →
    re-escalate toward `machine`
  - `--user 0` inside a container is contained-root (still contained unless a
    breach flag is also present).
  Intrinsic: pulling an image is a supply-chain capability (Part B), pinned by
  `IMG@sha256:…` or floating by tag.
- **interpreter** — `nested` = the nested language's behavior. If the language is
  parseable and we choose to model it (SQL: `DROP`=destroy, `SELECT`=observe),
  resolve; otherwise **opaque** (arbitrary `python -c …` → worst-case). This is
  why inline interpreters are not auto-approved today and stay that way by
  default; per-language sub-models are an opt-in refinement.
- **task-runner** — `nested` = project-config code = `ambient-config` provenance.
  Opaque unless we parse the recipe file (out of scope for v1) → worst-case.

## A.4 Resolution, recursion, termination

- **Resolvability test.** The nested computation is resolvable iff it is a literal
  string (or `find -exec` template) with no unexpanded `$var`/glob/stdin
  dependency for the *command identity* (arguments may still be argument-resolved,
  R5). `ssh host 'git pull'` resolvable; `ssh host "$CMD"` opaque.
- **Recursion.** Resolution re-enters the classifier on the nested string
  (`sudo ssh h 'sudo rm -rf /'` → three frames deep). Reuse the CST/parser — the
  nested string is parsed as a command line, then classified, then framed.
- **Depth bound.** Cap recursion depth (proposed 3). Beyond the cap → opaque. This
  also bounds pathological inputs.
- **Termination & worst-case.** Any unresolved link in the chain makes the whole
  delegate opaque from that point out → worst-case. Safe by construction.

## A.5 Interaction with the existing shell layer

The CST already classifies `A | B`, `$(…)`, backticks, and eval-safe
substitutions at the *shell* level. Delegation is the same idea pushed *inside a
command's arguments*: `bash -c 'STRING'`, `ssh host 'STRING'`,
`find -exec CMD \;` all carry a nested command line in an argument position. So
delegation resolution is "re-run the shell classifier on the argument," gated by
the delegator catalog knowing which argument holds the nested command. Much of
the machinery exists; what's new is the **frame algebra** and the **catalog**.

## A.6 Worked examples

- `timeout 60 rm -rf /data` → transparent frame, resolve `rm -rf /data` →
  `{destroy·machine/worktree·irreversible·unbounded}`. (The OmniFocus "timeout …"
  case: timeout is safe, the payload isn't — now expressible.)
- `sudo rm -rf /etc` → privilege frame over `rm -rf /etc`; authority=root makes
  the machine-locus destroy actually land, irreversible.
- `ssh deploy@prod 'systemctl restart api'` → remote frame; nested control on
  `remote`; intrinsic outbound/fixed + ambient secret.
- `docker run --rm alpine sh -c 'echo hi'` → isolation frame; nested contained →
  low. `docker run -v /:/host alpine rm -rf /host` → isolation breached by `-v /` →
  nested destroy re-escalated to `machine·irreversible`.
- `xargs rm` (stdin) → command `rm` known, arguments opaque → worst-case scale/
  locus → not auto-approved.
- `make deploy` → task-runner, recipe opaque → worst-case.

---

# Part B — Supply-chain provenance (refining F6 `network-sourced`)

## B.1 Why it must decompose

`network-sourced` lumps `curl https://x | sh` together with `go build`. But the
ecosystems give us **precise, describable facts** about *how much* fetched code
runs, *when*, from *where*, and *whether it's pinned*. That is exactly the
"many levels, each describable" granularity we want — and it's the point about
node packages: npm's model is well-documented, so we can classify it exactly.

## B.2 Sub-facets of fetched-code execution

A `network-sourced` execution capability decomposes into four sub-facets, each
named and defined:

- **source** — who published the code:
  `unverified-url` · `public-registry` (npm, PyPI, crates.io, Docker Hub — open
  publication) · `signed-repo` (apt/distro, maintainer-reviewed + signed) ·
  `private-registry` (org-configured) · `vendored` (already in-tree).
- **pinning** — how fixed the exact artifact is:
  `floating` (`@latest`, no lock) · `version` (a version, no hash) ·
  `hash-verified` (lockfile/`--require-hashes`/`go.sum` — exact bytes fixed) ·
  `digest` (`img@sha256:…`).
- **exec-surface** — when/what fetched code runs:
  `none` (download only, no code) · `install-hook` (arbitrary code on *install*:
  npm lifecycle, pip `setup.py`, gem extconf, dpkg maintainer scripts) ·
  `build-script` (code on *build*: cargo `build.rs`/proc-macros, node-gyp,
  `go generate`, cgo) · `call-time` (deps' code runs only when your program later
  runs — Go's normal model) · `run-artifact` (you execute the fetched binary/
  image).
- **isolation** — reuse R3: `none` (runs as your user, full access — npm/pip/
  cargo hooks) · `sandboxed` (container/VM — `docker run`).

A fetched-code capability is a point in source × pinning × exec-surface ×
isolation. Authority (R2) also composes (apt install hooks run as **root**).

## B.3 Ecosystem reference (the research)

Each row is a golden-set candidate; each fact needs a citation in the registry.

| command | source | pinning | exec-surface | isolation | notes |
|---|---|---|---|---|---|
| `curl https://x.sh \| sh` | unverified-url | floating | install-hook (runs now) | none | worst point |
| `npm install` | public-registry | floating→version | **install-hook** (pre/post/install, as user) | none | `--ignore-scripts` ⇒ exec-surface `none` |
| `npm ci` | public/private | **hash-verified** (lockfile integrity) | install-hook | none | still runs scripts unless `--ignore-scripts` |
| `pnpm install` / `yarn` | public/private | lockfile | install-hook | none | pnpm `enable-pre-post-scripts`; yarn berry stricter defaults |
| `pip install foo` | public-registry | version | **install-hook iff sdist** (`setup.py`); wheels don't run code | none | `--only-binary=:all:` ⇒ wheels ⇒ exec-surface `none` |
| `pip install --require-hashes -r req.txt` | public/private | **hash-verified** | as above | none | pinned + verified |
| `cargo build` | public-registry | **hash-verified** (`Cargo.lock` checksums; crates.io immutable) | **build-script** (`build.rs`, proc-macros) | none | `cargo fetch`/`add` = exec-surface `none` |
| `go build` | public-registry | **hash-verified** (`go.sum`) | **call-time** (no install/build hooks) | none | notably lower surface; `go generate`/cgo are the exceptions |
| `apt install foo` | **signed-repo** | version | **install-hook as root** | none | maintainer scripts, authority=root |
| `brew install foo` | public (core reviewed) / arbitrary taps | version | install-hook (formula is Ruby, as user) | none | taps widen source |
| `gem install foo` | public-registry | version | build-script (native extconf) | none | |
| `docker run img@sha256:…` | registry | **digest** | run-artifact | **sandboxed** | breaches via `-v`/`--privileged` (Part A) |
| `docker run img:tag` | registry | version (mutable tag) | run-artifact | sandboxed | tag is floating |

## B.4 How levels draw non-arbitrary lines

Because every point is describable, a level can state its acceptance precisely:

- A `developer` level might accept fetched-code execution iff
  `source ∈ {public-registry, signed-repo, private-registry, vendored}` **and**
  `pinning ≥ hash-verified` **and** `exec-surface ≤ build-script` — i.e. "I accept
  pinned, verified deps that run build scripts, from real registries." That admits
  `cargo build`, `go build`, `npm ci --ignore-scripts`,
  `pip install --require-hashes --only-binary=:all:`, but **not**
  `curl | sh` (unverified-url), **not** `npm install foo@latest` (floating +
  install-hook), **not** `docker run img:latest` (floating tag).
- A stricter `ci` level might additionally require `exec-surface ≤ none` (download
  only; run builds in a separate sandboxed step).
- The point: each admit/reject is justified by a named fact ("rejected: `pinning`
  is `floating`"), never a taste call.

## B.5 Per-artifact reputation is a different layer

The taxonomy classifies the **mechanism** (installing from npm with lifecycle
scripts), not the identity ("is `left-pad` safe"). Per-package reputation/known-
malware is an **orthogonal future data source** that could annotate a specific
artifact, layered on top. Keeping them separate preserves non-arbitrariness: the
mechanism is describable and stable; reputation is a separate, evolving feed.

---

# Feedback into the spec

- **F6 becomes faceted.** `network-sourced` is replaced by the source × pinning ×
  exec-surface (× isolation × authority) sub-model. `caller-inline`/`caller-file`/
  `ambient-config` stay as-is for local code.
- **Delegation is a modeled mechanism**, not a facet: a capability may wrap a
  nested profile under a frame; frames use **Authority** (R2) and **Isolation**
  (R3); unresolved nesting ⇒ opaque ⇒ worst-case.
- **The catalog and the ecosystem table are registry data** with `because` +
  evidence per row, and they anchor the golden-set.
- **Levels reference these facets directly**, which is what makes a `developer`
  level definable without arbitrary judgment.
