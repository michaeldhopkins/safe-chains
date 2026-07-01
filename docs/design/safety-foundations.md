# Safety foundations & the program

Status: draft (2026-07-01). Grounds the behavioral taxonomy in established
security theory (so its shape isn't invented), and scaffolds the wider program
the taxonomy makes possible.

## 1. Why ground it in theory

The non-arbitrariness rule (named-not-numbered, `because`, golden-set) keeps
*individual* classifications honest. But the *shape* of the taxonomy — which
facets exist — also needs to be non-arbitrary. The defense is that the facets are
not invented; they recapitulate ~50 years of protection theory. Each facet is a
restatement, for CLIs, of a known dimension of authority.

## 2. The facets are old ideas in CLI clothing

| our facet | established idea |
|---|---|
| Operation (observe/mutate/destroy/…) | **CIA triad** — Confidentiality (observe/disclosure), Integrity (mutate/destroy), Availability (destroy/control) |
| Locus (process→remote) | **blast radius / protection domain**; CVSS **Scope** (does it cross a boundary) |
| Reversibility | error-recovery / undo as a safety property; CVSS **Availability impact** |
| Disclosure (who learns data) | **Confidentiality**; information-flow control (who is the sink) |
| Network reach | CVSS **Attack Vector**; the classic local/remote boundary |
| Execution provenance | **provenance / supply-chain trust**; who authored the code that runs |
| Persistence (data/reconfigure/install) | **confused-deputy** surface; TOCTOU of trust; "install" = persistence in ATT&CK terms |
| Secret | Confidentiality of credentials; least-privilege for keys |
| Scale | blast radius magnitude |
| Authority (user/elevated/root) | CVSS **Privileges Required**; least privilege |
| Isolation | **object-capability** containment; sandboxing |

The two guiding principles the whole model serves:

- **Principle of Least Authority (POLA)** (Saltzer & Schroeder, 1975; the ocap
  community). A command should wield only the authority its purpose needs; a
  *level* is a user-chosen authority ceiling; the taxonomy is how we measure a
  command against that ceiling.
- **Object-capability (ocap) vs ambient authority** (Hardy; Miller, *Robust
  Composition*, 2006). A UNIX process runs with **ambient authority** — it may
  touch anything the user may. Our most dangerous facets are exactly the
  ambient-authority leaks: `execution=ambient-config` (runs code it wasn't handed),
  `network=outbound-arbitrary` (reaches anything), `persistence=reconfiguring`
  (rewrites future behavior). The safest tools (`wasmtime`, `pledge`/`unveil`,
  Capsicum, Landlock) replace ambient authority with explicit grants — the shape
  we want every CLI to move toward.

## 3. Saltzer & Schroeder, applied

Their eight principles map cleanly onto what safe-chains already does and where
the taxonomy takes it:

- **Fail-safe defaults** → allowlist-only; unknown ⇒ not approved; unresolved
  delegation ⇒ opaque worst-case; containment earned, not assumed.
- **Complete mediation** → every command checked, every segment, every redirect.
- **Least privilege** → the entire point of levels-as-authority-ceilings.
- **Open design** → the taxonomy and definitions are public and describable;
  no security by obscurity (and the reason the "models don't understand it yet"
  security was never acceptable).
- **Psychological acceptability** → the agent-facing feedback work (not-a-block,
  feedback-not-re-run); a user must be able to *reason* about a level, which is
  why every term is describable.
- **Economy of mechanism / separation of privilege / least common mechanism** →
  design constraints on the engine and on level composition.

## 4. The confused deputy is the through-line

Norm Hardy's confused deputy (1988): a privileged program is tricked into using
its authority for a less-privileged party. Every safe-chains vulnerability this
year was one:

- the agent writing `.safe-chains.toml` to widen its own trust (fixed v0.205.0);
- a redirect planting `.git/hooks/pre-commit` (fixed v0.206.0);
- `git config core.pager` / `jj config set git.executable-path` turning a config
  write into later code execution;
- the whole reason agents are dangerous: an agent is a deputy with the *user's*
  authority acting on *someone else's* (a repo's, a prompt's) intent.

The taxonomy names these surfaces (`persistence=reconfiguring/installing`,
`execution=ambient-config`, `locus=worktree-trusted`) so a level can refuse them.
This is the intellectual core: **safe-chains is a mediator that keeps a deputy
(the agent) from being confused into misusing the user's ambient authority.**

## 5. The program — four artifacts

The taxonomy is the seed of a larger body of work. Recording it here so the pieces
stay coherent.

### 5.1 safe-chains (the guard)
The runtime mediator. Consumes a compiled artifact; parses invocations; resolves
profiles (incl. delegation frames); evaluates the active level. Owns integration
with harnesses and the trusted-config boundary.

### 5.2 The safety database (the capability registry)
The structured, evidence-backed store: `tool → node → capabilities[]`, the
taxonomy definition, the level definitions, the delegator/breach/supply-chain
catalogs, and the golden-set. Its own project; validates every record
(`because` + evidence + golden-set green) and compiles the artifact safe-chains
links. Reusable beyond safe-chains (audits, docs, other guards).

### 5.3 The CLI-design database (derived — *good* API design)
A **byproduct** of classifying the world: as we measure tools, we surface where a
CLI is *unnecessarily* unsafe or inflexible relative to the value it provides, and
what safer affordance it could offer. A **design finding** record:

```
finding {
  tool: "rm"
  smell: irreversible-by-default
  because: "unlinks with no undo and no --dry-run; -f is one keystroke"
  safer-affordance: "trash-by-default with an explicit --unlink, and a -n preview"
  evidence: ...
}
```

Smell vocabulary (each derives mechanically from a capability pattern):
- **ambient-authority** — acts beyond what's designated (a fetcher that also
  writes files; implicit `~/.netrc`).
- **no-dry-run** — a `destroy`/`mutate` with no preview sibling.
- **irreversible-by-default** — easy `--force`, no `--force-with-lease` analogue,
  no trash/undo.
- **conflated-operations** — one subcommand does `observe` *and* `mutate`, so a
  read-only grant is impossible; the tool can't be run at reduced authority.
- **unpinnable / unverifiable** — a package/build tool with no lockfile, hashes,
  or `--ignore-scripts`.
- **deferred-exec-config** — config keys that become executable behavior
  (`core.pager`, `git.executable-path`) with no data/exec separation.
- **no-least-authority-mode** — no `--read-only`, `--no-network`, `--offline`, or
  sandbox flag; the tool is all-or-nothing.
- **confused-deputy-surface** — reads execution-affecting config from the CWD it
  operates on (`.envrc`, repo hooks, project config) without a trust boundary.
- **silent-scope-escalation** — `-r`/`-R`/globs that reach `unbounded` scale with
  no confirmation.

The ideal these point at is §2's ocap shape: deny-by-default, grant-by-flag,
separable read/write, previewable, reversible, pinnable. `git push
--force-with-lease`, `pip --only-binary --require-hashes`, `wasmtime --dir`,
`rm -i`, `terraform plan`/`apply` split are existing examples of the good shape.

### 5.4 The book (short) — a philosophy of CLI safety
A synthesis for a general technical reader. Working outline:

1. **Ambient authority and the shell.** Why a command line is a loaded gun: every
   process inherits the user's whole authority.
2. **Fifty years of the answer.** Saltzer-Schroeder, least privilege, the
   confused deputy, object capabilities — told through shell examples.
3. **A taxonomy of what commands do.** The facets as a lens; reading a command's
   true blast radius.
4. **Levels as authority ceilings.** Safety as a user choice, not a vendor verdict.
5. **Where authority leaks.** Delegation and the supply chain — nested execution,
   downloaded code, the frames that contain or amplify it.
6. **Containment.** Isolation as capability reduction; the ocap ceiling.
7. **Designing safer tools.** Affordances, dry-runs, least-authority modes, the
   design smells and their fixes.
8. **The agent era.** Autonomous agents as the ultimate confused deputy, and why
   external mediation (a describable, user-tunable authority boundary) is now
   load-bearing rather than optional.

## 6. How the artifacts feed each other

```
   world of CLIs ──classify──▶ safety database ──compile──▶ safe-chains
                                   │      │
                                   │      └──derive──▶ CLI-design database
                                   └──synthesize──▶ the book
```

The database is the hub: research goes in once, evidence-backed and describable,
and the guard, the design critique, and the philosophy all derive from it. That
is the payoff of moving from a lossy verdict to a recorded behavior — the research
stops being thrown away.
