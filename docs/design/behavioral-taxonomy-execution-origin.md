# Behavioral taxonomy — execution origin & the workspace-code scope

*Refinement of v1.4 §2.6 (execution/supply-chain). Status: design, 2026-07-14.*

## 1. The scope decision (endorsed)

safe-chains runs in the user's workspace (cwd/root context). For code execution we adopt an
explicit scope:

> safe-chains guarantees two things a **static** classifier actually can: **it never auto-approves
> running FOREIGN code, and it never auto-approves reaching OUT of the workspace.** It does **not**
> try to prevent execution of the workspace's *own* code — that is the dev loop, it cannot be bounded
> statically anyway, and it is *already* permitted (`cargo test`/`bench` run arbitrary project code).
> Preventing a malicious agent that stages-then-runs its *own* workspace code is the **sandbox /
> harness's** job (`Isolation` facet), not safe-chains'.

Why this is consistent, not a loosening:
- `cargo test`, `cargo bench`, and `build.rs` already execute arbitrary project code and are allowed.
  An agent can already run anything by writing it into a test and running `cargo test`. So denying
  `cargo run` / `python ./x.py` closes no real hole — it only adds dev-loop friction.
- The real, statically-decidable danger is **foreign** code (a downloaded script, `/tmp/x.sh`,
  `~/Downloads/evil.py`, `curl | sh`, an npm postinstall) and **reaching out of the workspace**.
  Those stay denied.

## 2. The model is already here (v1.4 §2.6)

Execution is already a first-class facet. `Operation::Execute` carries:

```
Execution { trust: ExecutionTrust, supply_chain: Option<SupplyChain> }
```

`ExecutionTrust` already encodes *how the code was supplied*:
- `SelfCode` — the tool runs its own built-in logic / the project's own artifact
- `CallerInline` — code the caller passed inline (`bash -c`, `python -c`, `perl -e`, a DSL string)
- `CallerFile` — code from a file the caller named (`bash script.sh`, `python x.py`, a run-artifact)
- `AmbientConfig` — code from ambient project config (Makefile, hooks, `.envrc`, plugins)
- `NetworkSourced` — downloaded code, detailed by `SupplyChain { source, pinning, exec_surface }`

The `dev` level already **admits** `cargo build` (`NetworkSourced`, `PublicRegistry`, `HashVerified`,
`BuildScript`) and **denies** `curl | sh` (`UnverifiedUrl`, `Floating`, `RunArtifact`). So provenance
already drives the verdict — for *network*-sourced code.

## 3. The gap

`CallerFile` and `CallerInline` carry **no locus for the executor**. `bash ./local.sh` and
`bash /tmp/evil.sh` are both `Execute + CallerFile` — indistinguishable — so the level cannot admit
the first while denying the second. The conservative consequence today: **all** script-file / project
execution denies (`cargo run`, `go run .`, `python ./x.py`, `bash ./x.sh`), which is exactly the
inconsistency this doc resolves.

`LocalLocus` (`Worktree` / `WorktreeTrusted` / `Temp` / `User` / `Machine` / `SandboxScope` / …) is the
existing facet for "where on this host." It already grades read/write reach. **The refinement is to
attach the executor's `LocalLocus` to `Execute` capabilities** — the locus of the *thing being run*,
not of a read/write operand.

## 4. The refinement — three facets, all pre-existing

| Facet | Meaning here | Source |
|---|---|---|
| `Operation::Execute` | the effect: runs code, unbounded effects | exists |
| `ExecutionTrust` | *how* code is supplied (self/inline/file/ambient/network) | exists |
| executor `LocalLocus` | *where* the supplied code lives (worktree vs foreign) | exists as a facet; **newly attached to Execute** |

Computing the executor locus:
- **Implicit-project runners** (`cargo run`, `go run`/`build`/`test`, `dotnet run`, `swift run`): the
  executor is the current project → `Worktree`. If redirected out (`cargo run --manifest-path
  ~/other/Cargo.toml`, `make -C /elsewhere`), the executor locus is *that* path's locus.
- **Explicit-path runners** (`bash FILE`, `python FILE`, `node FILE`, `ruby FILE`, `go run PATH`):
  executor = the path → `classify_locus(path)`.
- **Inline** (`bash -c`, `python -c`, `perl -e`, `mlr put`, `sed 'e'`): no path; see §5.
- **Unpinnable** executor (`$VAR`, glob, command-substitution, unresolvable): → worst-case (§6).

## 5. The rule (level admit for `Execute`)

The `dev` level admits `Execute` iff:

1. `trust ∈ {SelfCode, CallerFile}` **and** executor `LocalLocus ∈ {Worktree, WorktreeTrusted,
   SandboxScope}` — running the workspace's own code (the dev loop). **ALLOW.**
2. `trust == NetworkSourced` with an accepted supply chain (the existing §2.6 rule: public-registry /
   signed-repo, version/hash-pinned, build-script/call-time). **ALLOW** (`cargo build`).

It denies:

- executor `LocalLocus ∈ {User, Machine, Temp, Device, Kernel}` → **foreign code** → **DENY**.
  (`bash /tmp/x.sh`, `python ~/Downloads/x.py`, `cargo run --manifest-path ~/other`.)
  **Note the asymmetry:** `Temp` (`/tmp`) is a fine place to *write* (scratch) but **not** a trusted
  place to *run from* — running `/tmp/x.sh` is running code an agent staged or downloaded there. So
  `Temp` is admitted for write/read loci but **denied as an executor locus**.
- `trust == CallerInline` for a **non-shell** interpreter (`python -c`, `perl -e`, `node -e`, `mlr
  put`, `sed 'e'`) — opaque code we cannot inspect → **DENY**. **Exception:** *shell* inline
  (`bash -c 'CMD'`, `sh -c`) is not opaque — it is **re-parsed as a nested command** by the existing
  wrapper-revalidation path and inherits that command's verdict (`bash -c 'echo hi'` allows;
  `bash -c 'curl … | sh'` denies). Inline shell therefore stays on its current path, unchanged.
- `trust == NetworkSourced` with `UnverifiedUrl`/`Floating`/`RunArtifact` (`curl | sh`) or
  `InstallHook` (`npm install` postinstall, `pip install` setup.py) → **DENY** (existing).
- executor locus **unknown / unpinnable** → worst-case (`Machine`) → **DENY** (§6).

## 6. Fail conservatively

- Any executor we cannot **pin to a worktree locus** is treated as foreign → deny. A `$VAR` path, a
  glob, a command-substitution executor, an unresolvable relative path → worst-case → deny. (Mirrors
  the standing fail-closed-resolver rule.)
- Symlinks and `$PATH` remain **out of scope** (classified by literal spelling — a `./link` pointing
  outside reads as worktree). That is the sandbox/harness's problem, per §1 and `AGENTS.md` §0.2.

## 7. Per-command reclassification (before → after)

| Command | trust | executor locus | today | after |
|---|---|---|---|---|
| `cargo run`, `go run .`, `dotnet run`, `swift run` | SelfCode | Worktree | deny | **allow** |
| `cargo run --manifest-path ~/o/Cargo.toml` | SelfCode | User | deny | deny |
| `python ./x.py`, `node ./x.js`, `ruby ./x.rb`, `bash ./x.sh` | CallerFile | Worktree | deny | **allow** |
| `python /tmp/x.py`, `bash /tmp/x.sh` | CallerFile | Temp | deny | deny |
| `python ~/Downloads/x.py`, `go run ~/Downloads/x.go` | CallerFile | User | deny | deny |
| `bash -c 'echo hi'` | (shell inline → re-parsed) | — | allow | allow |
| `python -c '…'`, `perl -e '…'`, `mlr put '…'`, `sed 'e …'` | CallerInline | — | deny | deny |
| `cargo build/test/bench` | SelfCode/Network | Worktree/registry | allow | allow (now *principled*) |
| `curl … \| sh`, `npm install`, `pip install x` | NetworkSourced | — | deny | deny |

## 8. The staging-bypass is IN SCOPE (stated, tested)

`bash ./staged-by-the-agent.sh` → CallerFile @ Worktree → **allow**, by design. safe-chains does not
prevent an agent running its own workspace code; the sandbox does. A proptest **asserts this allows**,
so that any future tightening is a conscious change that visibly breaks the test — not a silent drift.

## 9. Proptest plan (how we establish this scope safely)

The scope is *established and locked by property tests*, enumerating the real code-exec command corpus
so new commands are auto-covered. Build these first (red), then the resolvers (green).

1. **Core safety invariant (the one that matters):** for EVERY code-exec command, a **foreign**
   executor denies. Fuzz executor paths from a foreign corpus (`/etc/x`, `~/x`, `~/Downloads/x`,
   `/tmp/x`, `../x`, `/usr/local/bin/x`, an absolute path outside cwd) × the command corpus (`bash`,
   `sh`, `python`, `node`, `ruby`, `go run`, `cargo run --manifest-path`, `perl`, …) → **all deny**.
2. **Locus monotonicity:** swapping a *worktree* executor for a *foreign* one only ever flips
   allow→deny, never the reverse. `verdict(cmd @ worktree) ≥ verdict(cmd @ foreign)` in the allow order.
3. **Family consistency:** `build`/`test`/`bench`/`run` of the same project classify identically
   (all `SelfCode @ Worktree`) — extends the `subcommand_families_share_core_flags` idea to *levels*.
4. **Inline opacity:** non-shell inline (`python -c`, `perl -e`, `mlr put`, `node -e`, `sed 'e'`)
   denies; shell inline (`bash -c`, `sh -c`) matches its **re-parsed inner** verdict.
5. **Fail-closed executor:** unpinnable executor (`$VAR`, glob, command-substitution, unresolvable)
   denies.
6. **Scope-documentation test:** a worktree staged script (`bash ./staged.sh`) **allows** — pins §8.

Each is a property over a `(command × executor-locus)` matrix, enumerated from the corpus, red→green
proven.

## 10. Rollout order — LANDED (2026-07-14)

Implemented, tested (§9 proptests all green + enforced), installed, dogfooded:

1. The executor locus rides on the `Execute` capability's `Locus.local` (no struct change needed —
   the field already exists). `execute_file_verdict(path)` / `execute_project_verdict()`
   (`engine/resolve.rs`) build an Execute cap with MODEST facets (the scope decision: the code's
   downstream effects are the sandbox's job, not attributed here) and project it.
2. Level algebra: a `[[level.developer.allow]]` clause with `operation=["execute"]`,
   `locus.local = ">= sandbox-scope, <= worktree-trusted"` (a two-sided range — new DSL form),
   `execution <= caller-file`. The LOCUS band is the discriminator; it excludes `temp` (below,
   ordinal `temp < sandbox-scope`) and `user`/`machine` (above) for free. Inline code is
   `process`-locus (below the band) → denied.
3. Wiring: `bash`/`sh` FILE (`handlers/shell.rs`); a declarative `executor = "file" | "project"`
   field on TOML **subs and fallbacks** (`dispatch::dispatch_executor`); interpreters
   python3/node/ruby (`file` fallback + shared `handlers/interpreter.rs`); `go run` (`file` sub);
   `cargo run` (`project` sub + `executor_redirect_flag = "--manifest-path"` locus-gating the
   redirect-out value). `-c`/`-e` inline stays denied (absent from the allowlist).
4. Property tests (`handler_property_tests.rs`): foreign-denies, worktree-allows (bash/sh/python/
   node/ruby/go run), locus-monotonicity, opaque-inline-denies, unpinnable-denies, family
   consistency (cargo build/test/bench/run), and manifest-path redirect gating — all green.
5. The `Isolation` facet stays the escape valve (unchanged): a sandboxed harness could later admit
   foreign/`Temp` executor loci clamped to `SandboxScope` — a per-harness decision, out of scope.

**Family-wide gating (landed):** `--manifest-path` is gated by EXECUTOR locus across the WHOLE cargo
family (build/test/bench/check/run/doc) via a `Role::Exec` path-gate (`--manifest-path = "exec"` in
cargo's `[command.path_gate]`) — a foreign manifest runs a foreign project's build.rs/tests/binary,
so it denies; a worktree/nested-crate manifest allows. `Role::Exec` gates by `execute_file_verdict`,
which (unlike `write`) denies `/tmp`. And `--config` — a `target.*.runner`/`build.rustc-wrapper`
command-injection surface with no allowlistable safe subset — is dropped from every cargo sub.
