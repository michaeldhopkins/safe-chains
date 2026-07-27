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

### Measured: Java, Go, Lua, Julia, R

These five were first enumerated from documentation and shipped as `measured = false`. The
toolchains were then installed (OpenJDK 26.0.2, go1.26.5, Lua 5.5.0, Julia 1.12.6, R 4.6.1) and every
row probed with a marker file. **Every documented claim held**, and one was found to be understated.

| Probe | Result |
| --- | --- |
| `JAVA_TOOL_OPTIONS='-javaagent:<jar>' java Hello.java` | agent's `premain` ran, marker written |
| `_JAVA_OPTIONS` / `JDK_JAVA_OPTIONS`, same agent | both ran — all three spellings execute |
| `JAVA_TOOL_OPTIONS='-Xmx16m -XX:OnOutOfMemoryError=<script>'` | script ran on heap exhaustion |
| `-XX:+CrashOnOutOfMemoryError -XX:OnError=<script>` | script ran on the fatal error |
| `CLASSPATH=<dir> java Run` | planted class loaded and ran |
| `GOFLAGS='-toolexec=<script>' go build` | script invoked in place of the toolchain |
| `GOFLAGS='-overlay=<json>' go run` | printed the **planted** source, not the file on disk |
| `GOFLAGS='-modfile=<alt>' go list -m` | reported the planted module name |
| `LUA_INIT='<lua source>'` and `LUA_INIT=@<file>` | **both** forms executed |
| `JULIA_LOAD_PATH=<dir>` with a planted module | `using Evil` loaded and ran it |
| `JULIA_DEPOT_PATH=<dir>` with `config/startup.jl` | ran at startup with **no import at all** |
| `R_PROFILE_USER=<file> Rscript -e …` | sourced and ran |

Two corrections came out of this:

- **`JULIA_DEPOT_PATH` is stronger than "package depot, from which code is loaded."** The depot
  *contains* `config/startup.jl`, so pointing it at a directory runs that file unconditionally at
  interpreter start — no `using`, no import, nothing the script has to do. The entry now says so.
- **`-XX:+ScavengeBeforeFullGC` is unrecognized on OpenJDK 26** (a ParallelGC-era flag). It is kept:
  an unrecognized VM option makes the JVM refuse to start, so it cannot become a route to execution,
  and users on older JDKs still get it. This is the "obsolete entries are fine if they stayed safe"
  rule in AGENTS.md.

The other 22 allowlisted JVM flags and all 14 allowlisted `GOFLAGS` entries were accepted without
warning, so the allowlists are neither over- nor under-stated.

Still unprobed, and still `measured = false`: `CORECLR_PROFILER_PATH` and `CORECLR_ENABLE_PROFILING`.
`dotnet` IS installed — these need a native profiler `.dylib` to observe, which a marker file cannot
stand in for. Also not yet researched at all: `GOPATH`/`GOPROXY`/`GOTOOLCHAIN`, `LUA_PATH`/`LUA_CPATH`,
`R_HOME`, and Deno's `DENO_DIR`/`DENO_AUTH_TOKENS` — none of which have entries in `envvars.toml`.

#### The JVM option string: why `-XX:` cannot be a prefix

The obvious way to write the Java allowlist is `-Xm*` plus a blanket `-XX:`, since `-XX` is where the
GC and JIT tuning lives and that is all anyone sets these variables for. It is wrong. Per the Oracle
`java` launcher reference, `-XX:OnError=<cmd>` and `-XX:OnOutOfMemoryError=<cmd>` run "a custom
command or a series of semicolon-separated commands" — **direct command execution wearing the same
prefix as the harmless flags.** So the tuning flags are listed one at a time, and a `-XX:` invented
later is denied until someone reads it.

`-D<name>=<value>` is also excluded, against the tuning literature that calls system properties
harmless. Properties reach security-relevant machinery — `java.rmi.server.codebase` historically
enabled remote class loading, `jdk.attach.allowAttachSelf` opens self-attach — and sorting the
harmless ones from the rest is per-property research nobody has done. Allowing `-D` wholesale on the
strength of "properties are just configuration" would be the global shortcut this project has
already decided against.

Excluded as code loaders: `-javaagent:`, `-agentlib:`, `-agentpath:`, `-Xbootclasspath/a:`,
`-cp`/`-classpath`, `--module-path`/`-p`, `--upgrade-module-path`, `@argfile`, `-XX:VMOptionsFile=`,
`-XX:CompilerDirectivesFile=`.

**The three are NOT one mechanism**, which is the easy assumption and a wrong one. Measured on
OpenJDK 26.0.2:

| Variable | Read by | `-cp ./lib` |
| --- | --- | --- |
| `JAVA_TOOL_OPTIONS` | the VM | `Unrecognized option`, JVM refuses to start |
| `_JAVA_OPTIONS` | the VM | `Unrecognized option`, JVM refuses to start |
| `JDK_JAVA_OPTIONS` | the **launcher** | loaded and ran a planted class |

So `JDK_JAVA_OPTIONS` is a strict superset: it takes launcher options the other two reject. Their
`allowed` lists are still kept byte-identical as a deliberate conservative choice — the intersection
— and a guard fails if a future edit touches one and not the others. `JDK_JAVA_OPTIONS` carries the
launcher-only path flags on top of that shared list.

#### Go, Lua, Julia

`GOFLAGS` is Go's `RUSTFLAGS`. `-toolexec` invokes an arbitrary program in place of the toolchain,
`-exec` runs the built binary through one, and `-overlay` maps disk paths to different backing files
— source substitution before compilation, with no flag that looks like it is loading anything.
`-ldflags`/`-gcflags` pass through to the linker and compiler and are excluded for the reason
`RUSTFLAGS`' `-C link-arg` is. Allowed: the test and build selectors (`-v -x -n -count -tags -race
-mod -short -timeout -run -skip -buildvcs -trimpath -json`).

`LUA_INIT` is `opaque` rather than a path, because it is Lua **source** unless it begins with `@`, in
which case the remainder is a file to run. One shape cannot express "code or a path depending on the
first byte", and Lua source is not something this classifier reads, so presence denies in both forms.

`JULIA_LOAD_PATH` and `JULIA_DEPOT_PATH` are ordinary `exec-path`: code is loaded from them, so they
follow the executor locus — in-worktree allows, `/tmp` and `~` do not.

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

## Build status

Steps 1–3 are BUILT (`envvars.toml` + `src/envvars.rs`, wired into `simple_verdict`). Every vector
measured above now denies, and unlisted names remain untouched.

The env-twin sweep found four more, all previously allowed, in each case with the danger already
documented in the command's own TOML:

| Flag form (already denied) | Env twin (was allowed) |
| --- | --- |
| `cargo build --config build.rustc-wrapper=…` | `RUSTC_WRAPPER=…`, `CARGO_BUILD_RUSTC_WRAPPER=…` |
| `cargo test --config target.<triple>.runner=…` | `CARGO_TARGET_<TRIPLE>_RUNNER=…` |
| (as above, linker) | `CARGO_TARGET_<TRIPLE>_LINKER=…` |
| `git -c core.pager='sh -c evil'` | `GIT_CONFIG_COUNT` + `GIT_CONFIG_KEY_n` + `GIT_CONFIG_VALUE_n` |

Two mechanisms were needed beyond the original three shapes:

- **Name globs.** `CARGO_TARGET_<TRIPLE>_RUNNER` and `GIT_CONFIG_KEY_n` carry a variable segment, so
  an exact-name table cannot reach them.
- **`shape = "opaque"`.** `GIT_CONFIG_VALUE_0` is a command when the paired `GIT_CONFIG_KEY_0` is
  `core.pager` and inert when it is `user.name`. The pairing is split across variables, so no single
  value can be certified and presence denies. That matches the flag spelling, which already denies
  `git -c user.name=x log` because the key is not on git's permitted list.

Worth recording about git's `-c`: it ALREADY classifies per key and recurses the value —
`git -c core.pager=cat log` allows while `git -c core.pager='sh -c evil' log` denies. The env work
mirrors an approach the codebase had already taken rather than inventing one.

Step 4 is also BUILT. `shape = "option-string"` parses the value as interpreter flags and requires
every token to match a researched-inert prefix; anything else denies. Allowlist-shaped, so a switch
invented tomorrow denies by not being listed — which matters because these variables carry the
loader, debugger and permission flags and a denylist would rot.

`-C key=value` and `-Ckey=value` are the same flag, so a lone short flag is joined to the following
token before matching. That is not what stops `-C linker=/tmp/evil` today (a bare `-C` is not
allowlisted, so the split form already denies) — it stops a FUTURE authoring error, where someone
adds `-C` to permit `-C opt-level=3` and unwittingly admits every `-C` key.

Now denied, all previously allowed: `RUSTFLAGS='-C linker=…'`, `CARGO_BUILD_RUSTFLAGS=…`,
`NODE_OPTIONS='--require …'`/`--import`/`--inspect=0.0.0.0:9229`/`--allow-child-process`,
`RUBYOPT='-r…'`, `PERL5OPT='-I… -M…'`, `PERL5OPT='-I… -d:…'`, `JAVA_TOOL_OPTIONS='-javaagent:…'`.

Still allowed: `RUSTFLAGS='-D warnings'`, `-C opt-level=3`, `RUSTDOCFLAGS='-D warnings'`,
`NODE_OPTIONS='--max-old-space-size=4096'`, `RUBYOPT='-w'`.

The JVM trio, `GOFLAGS`, `LUA_INIT` and the Julia paths were added from documentation afterwards and
later probed — see "Measured: Java, Go, Lua, Julia, R" above, in particular why `-XX:` is not a
prefix and `-D` is not allowlisted. `JAVA_TOOL_OPTIONS='-Xmx2g'` and `GOFLAGS='-mod=readonly -v'`
allow; the agent and toolexec vectors deny.

### `starts_with` was the wrong matcher

The first cut matched a token against the allowlist with `starts_with`, which is correct for a value
that GLUES onto its flag — `-Xmx2g`, `-Dwarnings`, `-verbose:gc` — and wrong for everything else. It
silently admitted any longer flag that happened to share a listed one's spelling:

    GOFLAGS='-modfile=/tmp/evil.mod' go list ./...      admitted by `-mod`
    GOFLAGS='-vet=off' go vet ./...                     admitted by `-v`

Neither flag was researched; both were admitted by an accident of naming. `-modfile` names an
alternate `go.mod`, so it redirects module resolution — and `go list` and `go vet` **are**
allowlisted, so this was reachable rather than theoretical. Later confirmed on go1.26.5:
`GOFLAGS='-modfile=<alt>' go list -m` reports the planted module, not the real one.

No mechanical rule separates the two cases. "The character after the match must not be a letter"
would fix `-mod`/`-modfile` and break rustc's `-D warnings`, which glues a letter for the same
reason. So the author states it: **an entry is a whole flag unless it ends in `*`.** `-mod` admits
`-mod` and `-mod=vendor` but not `-modfile=…`; `-Xmx*` admits `-Xmx2g`. Every `*` is a claim that
nothing else in that tool shares the spelling, which is a thing a reviewer can check.

`every_option_string_entry_obeys_its_own_star` walks the real table and asserts each entry does what
its spelling says — a starred entry accepts an extension of itself, a bare one rejects it — so a
future entry is covered without anyone remembering to add a case.

### The duplicate-key incident

Adding `LUA_INIT` a second time (as `opaque`, having already listed it as `exec-path`) made the whole
`envvars.toml` fail to parse, and **every listed variable denied** — including `-Xmx2g`. Two things
are worth keeping from that:

- The failure was **fail-closed and loud**: a broken table denies rather than allows, and
  `the_table_parses_and_covers_the_measured_vectors` fails outright. TOML's duplicate-key rejection
  is doing real work here; a format that silently took the last definition would have left one
  variable quietly misclassified instead.
- It was found by running the BINARY before running the suite. The suite had it. Probe the built
  binary to confirm behaviour, not to discover it.

### A path-valued flag is gated, not excluded

Review found `-Cincremental` sitting in RUSTFLAGS' `allowed` list. `allowed` matches the flag NAME
and never looks at the value, so:

    RUSTFLAGS='-Cincremental=/etc/cron.d' cargo build     allowed
    cargo build --target-dir /etc/x                       denied

— the env twin of a guarded flag, reintroduced inside the file built to close that class.

The first fix was to remove the flag and forbid path-valued entries outright, on the grounds that
classifying the value would need a path-detector heuristic, which §0 forbids. **That was an
over-correction and it was wrong.** §0 forbids a *heuristic* detector where unrecognized input is
safe by omission. Declaring in the TOML that a named flag takes a write path, and routing that value
through the existing locus rules, is explicit and fail-closed — the opposite of a hidden denylist.

So an option-string entry may now carry `path_flags`, spelled as the registry already spells the
same idea (`PathFlag` / `PathRole`, roles `read` and `write`):

    path_flags = [{ flag = "-Cincremental", role = "write" }]

The value goes through `write_target_verdict`, so the env spelling and the command-line spelling give
the same answer: `./target/inc` allows, `/etc/cron.d` denies. `-XX:HeapDumpPath` is declared the same
way, which is what makes `-XX:+HeapDumpOnOutOfMemoryError` usable without handing out an arbitrary
write target for a 15MB image of process memory.

Three properties keep it honest, each red-demoed:

- **It opens nothing by itself.** An undeclared flag is still judged by `allowed`, where it is
  absent, so it denies — with or without a value. `-XX:VMOptionsFile`, `-Cprofile-use` and
  `-overlay` are unaffected.
- **A declared flag must not also be in `allowed`.** `allowed` is checked second and matches on the
  name alone, so a flag in both would reach the ungated branch. A structural test walks the real
  table and fails if any entry does that.
- **No value means no gate, so it denies.** `-Cincremental` and `-Cincremental=` are both refused
  rather than treated as a bare flag.

### An assignment's level, not just its denial

`cst/check.rs::simple_verdict` computed the env verdict, checked it for `Denied`, and threw the
LEVEL away. So a listed assignment could deny, but never make an otherwise-inert invocation count as
a write or an execution:

    touch ./x                               DENY at paranoid
    RUSTFLAGS='-Cincremental=./x' echo hi   ALLOW at paranoid   ← the same worktree write
    PYTHONPATH=./lib echo hi                ALLOW at reader     ← worktree code on the import path

Fixed by combining the env verdict into `sub_v` rather than special-casing its denial, so every
return path carries it. The env spelling and the command spelling now classify identically, and the
guard asserts the LEVEL rather than allow/deny — an allow/deny assertion passes either way, which is
why the gap survived the first round of tests.

`PYTHONPATH`/`CLASSPATH` land at `developer` rather than `editor`, which is right: putting code where
an interpreter will import it is an execution capability, not a write.

### The classpath flags: one capability, two spellings

`CLASSPATH` is an `exec-path` entry, so `CLASSPATH=./lib mvn test` allows and `CLASSPATH=/tmp/evil
mvn test` denies. The flag spelling denied unconditionally:

    CLASSPATH=./lib mvn test                     allowed
    JDK_JAVA_OPTIONS='-cp ./lib' mvn test        denied     ← same capability

Live rather than theoretical: `mvn test`, `gradle build`, `./gradlew build` and `java -version` all
auto-approve, and all launch a JVM that reads these variables.

Fixed with a `path_flags` entry at `role = "exec"`, which routes the value through
`execute_file_verdict` — the same function the `exec-path` shape uses, so the two spellings cannot
disagree by construction. A classpath is a `:`-separated LIST and is gated element-wise, so
`-cp ./lib:/tmp/evil` denies on the second entry.

**Only `JDK_JAVA_OPTIONS` carries them**, and that asymmetry is measured, not stylistic: `-cp` is a
launcher option, so in the VM-read pair it is `Unrecognized option` and the JVM refuses to start.
Listing it there would have allowed a form that cannot run.

Two separator styles had to be added to express this, and each entry declares its own `sep`, measured
per flag: `-cp <path>` is space-separated and `-cp=<path>` is rejected, while `--module-path=<path>`
is the reverse. A space-separated flag will not consume a `-`-prefixed token as its value — `java`
rejects that outright ("`-cp` requires class path specification"), and swallowing it would take a
token out of the flag path unvalidated.

Colon-joined flags (`-Xbootclasspath/a:<path>`) exist, but no entry declares one, so that variant is
deliberately absent rather than written ahead of a use: an unexercised branch in a classifier is a
branch nothing has ever checked.

## Remaining

Nothing from the original plan, and the ecosystem sweep is done. What is left is coverage rather than
mechanism:

- **The CoreCLR profiler pair** — `CORECLR_PROFILER_PATH` and `CORECLR_ENABLE_PROFILING` are the
  only rows still `measured = false`. `dotnet` is installed; what a marker file cannot stand in for
  is a native profiler `.dylib`, which is what the pair actually loads.
- **Variables with no entry yet** — `GOPATH`/`GOPROXY`/`GOTOOLCHAIN`, `LUA_PATH`/`LUA_CPATH`,
  `R_HOME`, `DENO_DIR`/`DENO_AUTH_TOKENS`. These are candidates the sweep named but never researched,
  so they are ignored like any unlisted name. New research, not validation.
- **Widening the option-string allowlists** as real usage turns up inert flags that were omitted. A
  missing entry costs one approval, never a wrong verdict — which is why erring narrow is right for
  the rows that were never measured.

## Original build order

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
