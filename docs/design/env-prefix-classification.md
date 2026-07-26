# Env-prefix classification — findings, and why it is not built

Status: **research COMPLETE, build not started.** Recorded 2026-07-26. Every claim below about what
executes was measured on this machine by running a marker file, not taken from documentation — which
was wrong in at least one consequential place (see the NODE_OPTIONS note).

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
| `PYTHONBREAKPOINT=<mod>.<callable>` | **executes** — arbitrary callable invoked by `breakpoint()` |
| `PHPRC=<php.ini>` with `auto_prepend_file` | **executes** before every script |
| `PHP_INI_SCAN_DIR=<dir>` with `auto_prepend_file` | **executes** |
| `RUSTC_WRAPPER=<script> cargo build` | **executes** — invoked in place of rustc |
| `RUSTFLAGS='-C linker=<script>'` | **executes** — invoked as the linker |
| `DOTNET_STARTUP_HOOKS=<assembly>` | honored — host attempts the load and throws on a missing file |
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

## Option-string allowlists — researched 2026-07-26

Each interpreter was probed directly: a switch was placed in the variable and the interpreter run,
recording whether it was refused, accepted, or observed to execute injected code.

### RUBYOPT (ruby 3.4.2)

Ruby enforces its own subset. **`-e` is REFUSED**, which removes the obvious eval vector.

- accepted: `-w -W -v -d -r -I -E -U -T --jit --yjit --enable= --disable= --debug --verbose`
- refused: `-e -a -c -n -p -s -x -y -C -F -i -l -S`

Code vectors, both taking a PATH — so both classify under the exec-locus rule with no new judgement:

- `-r<file>` loads and executes. Measured: `RUBYOPT="-r<file>"` printed the marker before main.
- `-I<dir>` extends the load path, which feeds `-r` and can shadow a stdlib module a later
  `require` picks up.

Allowlist candidates (inert w.r.t. code loading): `-w -W -v --verbose --debug -E -U -T --jit
--yjit`. `--enable=`/`--disable=` toggle features rather than load code, but their VALUE should be
checked rather than assumed.

### PERL5OPT (perl 5.34.1)

Also a subset. **`-e`/`-E` REFUSED.**

- accepted: `-w -W -d -t -T -C -I -M -m -U -D`
- refused: `-e -E -X -c -n -p -l -a -F -s -S -i`

Code vectors, and the shapes differ in a way that matters:

- `-M`/`-m` take a MODULE NAME, not a path — `-M/path/to/Inject` errors with "Module name required".
  So alone they load from the standard `@INC`, i.e. installed code.
- `-I<dir>` is therefore the real lever: it points `@INC` at attacker-chosen code, and `-I<dir>
  -MInject` executed the marker.
- `-d:Module` loads `Devel::Module` — a SEPARATE vector from `-M`. Measured: `-I<dir> -d:Inj`
  executed the marker.
- `-U` permits unsafe operations.

Allowlist candidates: `-w -W -t -T -C -D` (`-t`/`-T` are taint modes and make things stricter).

### NODE_OPTIONS (node v22.17.0) — the least restricted, and three distinct danger classes

Node refuses very little: only `--eval`/`--print` (plus `--check`, `--input-type` and an entry point
per its docs). **The documentation's claim that `--require`, `--import` and `--experimental-loader`
are disallowed is FALSE on this version — all three were accepted, and `--require`/`--import` were
observed executing an injected module before main.**

1. **Code loading** (path values → exec locus): `--require`, `--import`, `--experimental-loader`,
   `--loader`.
2. **Network-exposed debugger** (→ remote code execution): `--inspect`, `--inspect-brk`,
   `--inspect-port`. `--inspect=0.0.0.0:9229` was accepted, which grants full execution to anyone
   who can reach the port. This class has no analogue in the other interpreters.
3. **Permission-model relaxation**: `--experimental-permission`, `--allow-child-process`,
   `--allow-fs-write`, `--allow-worker`, `--allow-addons` — all accepted. These REMOVE restrictions
   the program may be relying on.

Allowlist candidates: the V8 tuning and diagnostics flags — `--max-old-space-size`,
`--enable-source-maps` (both measured working) and similar. This is the variable needing the most
careful list, because the accepted set is nearly everything.

### Rust — MEASURED, both execute

Built a throwaway crate and ran cargo against it:

- `RUSTC_WRAPPER=<script> cargo build` — the wrapper was invoked. Cargo runs it in place of rustc,
  so it is a plain command substitution.
- `RUSTFLAGS='-C linker=<script>' cargo build` — the script ran as the linker. `-C link-arg`,
  `--extern` and `-Z` are the same family.

### PHP — MEASURED, both execute (after a corrected probe)

`PHPRC` names a php.ini and `PHP_INI_SCAN_DIR` names a directory of them; an ini setting
`auto_prepend_file=<file>` executes that file before every script. Both injected.

Worth recording the near-miss: the FIRST probe reported no injection, because `PHPRC` was pointed at
a directory rather than the ini file AND `auto_prepend_file` does not apply to `php -r`. A negative
probe result has to be validated (here: `php --ini` confirming the file was loaded) or it records a
false "safe".

### Python — a second vector beyond PYTHONPATH

`PYTHONBREAKPOINT=<module>.<callable>` names an arbitrary callable invoked by `breakpoint()`.
Measured: with the module on `PYTHONPATH`, the callable ran. Distinct from the `sitecustomize`
vector, and it needs no file placed in a magic location.

`PYTHONHOME` and `PYTHONEXECUTABLE` relocate the interpreter's own root and are enumerated but
unprobed.

### .NET — startup hooks honored

`DOTNET_STARTUP_HOOKS=<assembly>` — the host attempted the load and threw
`System.ArgumentException: Startup hook assembly … failed to load`, so the variable is honored and a
real assembly would execute. `CORECLR_ENABLE_PROFILING` + `CORECLR_PROFILER_PATH` (loads a native
profiler library) produced no observable signal via `dotnet --info`; enumerated, NOT verified.
`DOTNET_ADDITIONAL_DEPS` is the same family and unprobed.

### Not probed — toolchain absent on this machine

Enumerated from documentation; each needs the same marker-file treatment somewhere it is installed.

| Ecosystem | Variables | Why it matters |
| --- | --- | --- |
| Java | `JAVA_TOOL_OPTIONS`, `_JAVA_OPTIONS`, `JDK_JAVA_OPTIONS`, `CLASSPATH` | `-javaagent:<jar>`, `-agentlib:`, `-agentpath:`, `-Xbootclasspath/a:` load code into EVERY JVM started while set |
| Go | `GOFLAGS`, `GOPATH`, `GOPROXY`, `GOTOOLCHAIN` | `GOFLAGS='-toolexec=<binary>'` makes the toolchain invoke an arbitrary binary — the direct analogue of `RUSTC_WRAPPER` |
| Lua | `LUA_INIT`, `LUA_PATH`, `LUA_CPATH` | `LUA_INIT` executes its contents (or `@file`) at interpreter start |
| R | `R_PROFILE_USER`, `R_HOME` | profile file sourced at startup |
| Julia | `JULIA_LOAD_PATH`, `JULIA_DEPOT_PATH` | load-path injection |
| Deno | `DENO_DIR`, `DENO_AUTH_TOKENS` | cache poisoning / credential |

The pattern across every ecosystem is the same three shapes — run this command, load code from this
path, or read these interpreter flags — which is why the classification plan does not grow with the
number of languages. Each new ecosystem adds ROWS, not new rules.

## The pattern to sweep for: a gated flag with an ungated env twin

Two confirmed in cargo alone, and the danger was ALREADY documented in `cargo.toml` both times:

    cargo build --config build.rustc-wrapper=/tmp/evil        DENY
    RUSTC_WRAPPER=/tmp/evil cargo build                       ALLOW
    CARGO_BUILD_RUSTC_WRAPPER=/tmp/evil cargo build           ALLOW

    cargo build --config build.rustflags=["-Clinker=/tmp/evil"]   DENY
    RUSTFLAGS='-C linker=/tmp/evil' cargo build                   ALLOW
    CARGO_BUILD_RUSTFLAGS='-C linker=/tmp/evil' cargo build       ALLOW

Cargo mirrors its whole config surface into `CARGO_*`, so this is systematic there rather than two
oversights. Other tools with the same habit (env mirrors of config keys) should be checked the same
way — the search is "a flag we deny that has an environment spelling".

## Suggested build order

1. **Command-valued variables** — `GIT_SSH_COMMAND`, `GIT_PAGER`, `EDITOR`, `LESSOPEN` and the rest.
   Recurse through `command_verdict`. No new research: verified that `cat` allows and `sh -c evil`
   denies.
2. **Path-valued variables** — `LD_PRELOAD`, `PYTHONPATH`, `RUBYLIB`, `GIT_DIR`, `BASH_ENV`. Use the
   pathgate `exec` role for anything supplying code, `read`/`write` for data. No new research:
   verified worktree allows and `/tmp`/`~` deny under the exec role.
3. **The env-twin sweep** — find flags already denied that have an environment spelling. Two known
   in cargo; the search generalises.
4. **Option-string variables** — `RUBYOPT` and `PERL5OPT` first, since their accepted sets are small
   and their code vectors are path-shaped (so step 2's rule does most of the work). `NODE_OPTIONS`
   last: it accepts nearly everything and carries three unrelated danger classes.

Steps 1–3 need no judgement that has not already been made elsewhere in the classifier. Step 4 is
where the genuinely new allowlists live.
