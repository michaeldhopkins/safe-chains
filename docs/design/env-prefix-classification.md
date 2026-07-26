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
| `DYLD_INSERT_LIBRARIES=/tmp/evil.dylib <user-installed tool>` | the same, on macOS — but see the platform note below |
| `GIT_SSH_COMMAND='sh -c evil' git status` | git runs that command instead of `ssh` |
| `PATH=/tmp/evil ls` | every binary name resolves inside an attacker-chosen directory |
| `GIT_DIR=/tmp/evil git log` | retargets which repository is read |

`grep -rn 'LD_PRELOAD\|DYLD_INSERT\|GIT_SSH_COMMAND' src/` returns nothing: no env name appears
anywhere in the source.

**Platform note, measured 2026-07-26 on macOS 15 with SIP enabled.** Loader injection is not uniform,
and an earlier draft of this note got the example wrong. `DYLD_INSERT_LIBRARIES` is STRIPPED for
system binaries — pointing it at a nonexistent dylib and running `/bin/ls` produces no complaint at
all — so the `cat`/`ls` examples do not inject here. It IS honored for user-installed binaries: the
same probe against `~/.cargo/bin/safe-chains` terminated with `dyld: terminating because inserted
dylib could not be loaded`. Since an agent's toolchain is overwhelmingly Homebrew/cargo/npm rather
than `/bin`, this narrows the example without narrowing the exposure. `LD_PRELOAD` proper is a Linux
concern (CI, Linux users), and the non-loader vectors below — `PATH`, `GIT_SSH_COMMAND`, `GIT_DIR` —
depend on no OS protection and work everywhere. The `PATH` case is worth singling out because AGENTS.md §scope already
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

## The design that works: inspect a listed set, classify with existing rules

Revised 2026-07-26. The earlier framing — "an undeclared env name does not auto-approve" — is an
allowlist over an UNBOUNDED set (any program may read any variable), which is why it kept producing
unacceptable friction. Invert it:

- **Inspect only a listed set of variables.** That set is enumerable because it is defined by OS
  loaders, shell standards and each tool's documentation — not by user creativity.
- **Classify the VALUE with a rule we already have**, rather than judging the name.
- **Everything else is untouched.** `FOO=bar ls` keeps working; absence of a variable changes
  nothing. No new denials outside the list.

Nothing is "forbidden". `GIT_DIR=/tmp/evil` denies because `/tmp` is out-of-worktree under the
existing locus rules, not because `GIT_DIR` is on a naughty list.

### The rules already exist

| Value shape | Existing machinery | Verified behaviour |
| --- | --- | --- |
| a COMMAND | `command_verdict(value)` — the join-and-recurse `dispatch.rs:258` already does for `sudo X` | `cat` → allow, `sh -c evil` → deny, `vim` → allow |
| a path supplying CODE | pathgate role `exec` (as `cargo --manifest-path`) | worktree → allow, `/tmp` → deny, `~` → deny |
| a path read/written as DATA | pathgate roles `read` / `write` | worktree → allow, `/etc/cron.d` → deny |
| an OPTION STRING | parse as that interpreter's flags; needs a researched allowlist per interpreter | see below |

Recursion dissolves the value-sensitivity problem noted earlier: `GIT_PAGER=cat` and
`GIT_PAGER='sh -c evil'` classify correctly with no allowlist, because "is this string a safe
command" is a question already answered. Note one honest cost — `GIT_SSH_COMMAND='ssh -v'` denies,
because bare `ssh` is not allowlisted. Erring closed, but not friction-free.

## Measured injection vectors (all executed on this machine, 2026-07-26)

Each was proved by running a marker file, not inferred from documentation:

| Vector | Result |
| --- | --- |
| `NODE_OPTIONS="--require <file>"` | marker printed before main — **executes** |
| `NODE_OPTIONS="--import file://<file>"` | marker printed before main — **executes** |
| `RUBYOPT="-r<file>"` | **executes** |
| `PERL5OPT="-I<dir> -MInject"` | **executes** |
| `PYTHONPATH=<dir>` with `sitecustomize.py` | **executes** (Python auto-imports it at startup) |
| `BASH_ENV=<file>` with `bash -c` | **executes** |
| `PYTHONSTARTUP=<file>` with `python3 -c` | did NOT execute — interactive REPL only |
| `DYLD_INSERT_LIBRARIES` against `/bin/ls` | stripped by SIP — no effect |
| `DYLD_INSERT_LIBRARIES` against `~/.cargo/bin/…` | honored — loader aborts on a missing dylib |

**Methodology note, and a caution for whoever picks this up.** The Node documentation states that
`--require` and `--import` are disallowed in `NODE_OPTIONS` "for security reasons". On Node v22.17.0
that is false: both load and execute. This note previously repeated the doc claim and nearly
retracted a correct finding on the strength of it. Test the vector; do not trust the manual.

## Enumeration by ecosystem

**Verified as code-execution vectors.** Each needs value classification, not a boolean.

| Ecosystem | Variables | Value shape |
| --- | --- | --- |
| loader | `LD_PRELOAD`, `LD_AUDIT`, `LD_LIBRARY_PATH`, `DYLD_INSERT_LIBRARIES`, `DYLD_LIBRARY_PATH`, `DYLD_FRAMEWORK_PATH` | code path (`exec` locus) |
| shell | `BASH_ENV`, `ENV`, `PATH`, `IFS`, `SHELL` | code path / resolution order |
| node | `NODE_OPTIONS`, `NODE_PATH`, `NODE_REPL_EXTERNAL_MODULE`, `NODE_EXTRA_CA_CERTS`, `NODE_TLS_REJECT_UNAUTHORIZED` | option string / code path / trust anchor |
| ruby | `RUBYOPT`, `RUBYLIB`, `GEM_HOME`, `GEM_PATH`, `BUNDLE_GEMFILE` | option string / code path |
| python | `PYTHONPATH`, `PYTHONHOME`, `PYTHONSTARTUP` (interactive only) | code path |
| perl | `PERL5OPT`, `PERL5LIB`, `PERL5DB` | option string / code path |
| git | `GIT_SSH_COMMAND`, `GIT_SSH`, `GIT_EXTERNAL_DIFF`, `GIT_EDITOR`, `GIT_SEQUENCE_EDITOR`, `GIT_PAGER`, `GIT_ASKPASS`, `GIT_PROXY_COMMAND` | command |
| git | `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_OBJECT_DIRECTORY`, `GIT_ALTERNATE_OBJECT_DIRECTORIES`, `GIT_COMMON_DIR`, `GIT_CONFIG_GLOBAL`, `GIT_CONFIG_SYSTEM`, `GIT_CONFIG_COUNT`/`_KEY_n`/`_VALUE_n` | data path / config injection |
| cargo | `RUSTC_WRAPPER`, `RUSTC`, `CARGO_BUILD_RUSTC_WRAPPER`, `RUSTFLAGS`, and the whole `CARGO_*` config mirror | command / option string |
| pager | `PAGER`, `LESSOPEN`, `LESSCLOSE`, `EDITOR`, `VISUAL` | command |
| jvm | `JAVA_TOOL_OPTIONS`, `_JAVA_OPTIONS`, `CLASSPATH` | option string / code path |

### The cargo finding, which motivates the whole exercise

safe-chains ALREADY classifies `build.rustc-wrapper` as a code-execution key: `cargo build --config
build.rustc-wrapper=/tmp/evil` is in `examples_denied` and denies. The env spelling of the same
capability does not:

    cargo build --config build.rustc-wrapper=/tmp/evil   DENY
    RUSTC_WRAPPER=/tmp/evil cargo build                  ALLOW
    CARGO_BUILD_RUSTC_WRAPPER=/tmp/evil cargo build      ALLOW

The danger is not unknown here — it is documented in cargo.toml's own description. Only the spelling
went unchecked. Cargo mirrors its entire config surface into `CARGO_*`, so every guarded `--config`
key has an unguarded env twin. That pattern (a flag we gate, an env var we do not) is the thing to
search for across the registry.

## Option-string allowlists — the remaining research

Three variables carry a FLAG STRING for an interpreter, so classifying them means parsing that
interpreter's flags and allowing a researched subset. Verified dangerous flags:

- `NODE_OPTIONS` — `--require`/`-r` and `--import` both execute (measured). `--experimental-loader`
  and `--loader` are the same shape. Benign example measured: `--max-old-space-size=512`.
- `RUBYOPT` — `-r` executes (measured). `-I` extends the load path, so it feeds `-r`.
- `PERL5OPT` — `-M`/`-m` load a module, `-I` extends `@INC`; the pair executes (measured). `-d`
  starts the debugger.

What is NOT yet researched: the full permitted set for each, and which remaining flags are inert
enough to allowlist. That is the real work item, and it is per-interpreter rather than per-command.
`JAVA_TOOL_OPTIONS` and `RUSTFLAGS` are the same shape and unexamined.

## Suggested order

1. Land the two shapes that need NO new research, since both reuse existing rules end to end:
   command-valued variables via `command_verdict`, and path-valued ones via the pathgate roles.
2. Sweep the registry for flags we already gate that have an env twin (`RUSTC_WRAPPER` is one; there
   will be more).
3. Only then take the option-string allowlists, one interpreter at a time.
