# Env-prefix classification — findings, and why it is not built

Status: **research only. Not scheduled.** Recorded 2026-07-26 so the evidence is not lost.

A command may carry environment assignments before its name — `VAR=value cmd args`. safe-chains
does not classify them. This note records what that costs, what a fix would need, and the reason the
research is not close to sufficient to attempt one.

## The live fail-open

The classifier inspects env **values** for substitutions but never the **name**. `FOO=$(rm -rf /) ls`
correctly denies — the value carries a substitution — while a hostile name with a literal value is
invisible. All of these auto-approve today:

| Invocation | What the assignment does |
| --- | --- |
| `LD_PRELOAD=/tmp/evil.so ls` | loads an arbitrary shared object into the process |
| `DYLD_INSERT_LIBRARIES=/tmp/evil.dylib cat notes.txt` | the same, on macOS |
| `GIT_SSH_COMMAND='sh -c evil' git status` | git runs that command instead of `ssh` |
| `PATH=/tmp/evil ls` | every binary name resolves inside an attacker-chosen directory |
| `GIT_DIR=/tmp/evil git log` | retargets which repository is read |

`grep -rn 'LD_PRELOAD\|DYLD_INSERT\|GIT_SSH_COMMAND' src/` returns nothing: no env name appears
anywhere in the source. The `PATH` case is worth singling out because AGENTS.md §scope already
commits to worst-casing a name reached via a non-standard path (`./cat`, `/tmp/cat`); redefining
`PATH` achieves the same shadowing without the spelling that triggers the guard.

## An env assignment is a facet modifier

The useful framing is not "which names are safe" but "which facet does this move". `GIT_SSH_COMMAND=`
does not add an argument to `git status`; it changes the invocation's `execution` facet from "runs
git" to "runs a caller-supplied command". Git's ~45 documented variables sort onto existing axes:

| Class | Examples | Facet |
| --- | --- | --- |
| runs an external command | `GIT_SSH_COMMAND`, `GIT_EXTERNAL_DIFF`, `GIT_EDITOR`, `GIT_PAGER`, `GIT_ASKPASS` | `execution` |
| relocates the repository | `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_OBJECT_DIRECTORY` | `locus` |
| relocates configuration | `GIT_CONFIG_GLOBAL`, `GIT_CONFIG_SYSTEM` | `execution` — config defines aliases and `core.pager`, so a caller-controlled config file is code execution |
| diverts output | `GIT_TRACE=/path` | `locus` / `disclosure` |

So the work is classification, which this project already knows how to do — not enumeration of a set
that has no boundary.

## Measured blast radius of enforcing

| Surface | Result |
| --- | --- |
| `examples_safe` + `examples_denied` over 1,603 commands | 1,411 examples, **0** carrying an env prefix |
| string literals in `src/` and `tests/` | 14,132 scanned, **15** env-prefixed, **9** currently auto-approving |

The nine are two real names (`RACK_ENV`, `RAILS_ENV`) against one command family (`bundle`), three
synthetic `FOO` fixtures that exist to test env parsing, and two statement assignments (below).
Regression risk inside this repo is small and enumerable.

That number bounds OUR tests. It says nothing about user friction, and nothing here can: the repo
only knows the forms we thought to write down.

## Constraint any design must respect

`VAR=x; cmd` and `VAR=x cmd` are different mechanisms and must not be swept together.

- `GEM=./data; cat $GEM/notes.txt` is a **statement assignment**, parsed by `statement_assignments`
  (`cst/check.rs`), and it feeds the variable-pinning machinery that resolves `$GEM` to a known path.
  Several redirect and hot-operand tests depend on it.
- `VAR=x cmd` is an **env prefix**, held in `SimpleCmd.env`.

A change must target `SimpleCmd.env` only. Keying on "the command contains `=`" would break path
pinning.

## Why it is not built

Enforcing "an undeclared env name does not auto-approve" is attractive because the dangerous set
never needs enumerating — `LD_PRELOAD` and every name yet invented are denied by omission, with no
denylist. The obstacle is the other side of the ledger: it would start denying far more than it
starts allowing, and we cannot presently say by how much.

The prerequisite is per-command env-surface research, and it has not been scoped. A worked example of
how easy it is to get wrong: "declare `NODE_ENV` for npm" sounds like a reasonable seed and is not
one. `npm run` executes package.json scripts, `npm_config_*` variables override configuration
including the registry, and `NODE_OPTIONS` accepts `--require`, which loads an arbitrary module into
node before any script runs. A partial declaration there would look complete and be wrong.

Some variables are also value-sensitive rather than name-sensitive — `GIT_PAGER=cat` is inert,
`GIT_PAGER='sh -c evil'` is remote code execution. That is the same shape as the endpoint-flag
problem, so `eval_safe_flag_values`-style value constraints would be reusable, but it means a
declaration is sometimes name + permitted values rather than a name alone.

## If it is picked up later

1. Scope the per-command env surface for one heavyweight family first (`git` or `npm`), end to end,
   including value-sensitive cases. That establishes the real cost per command before any commitment.
2. Decide whether the universal, runtime-interpreted variables (`LD_PRELOAD`, `PATH`, `IFS`) belong
   in per-command declarations at all. They are properties of the process rather than of any program,
   so 1,603 omissions may be the right ANSWER by the wrong MODEL.
3. Only then consider the default. The security fix and the friction change should land separately.
