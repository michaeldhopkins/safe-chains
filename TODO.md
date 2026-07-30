# TODO

## Env prefixes — SHIPPED

`VAR=value cmd` classification is built (`envvars.toml` + `src/envvars.rs`); the original fail-opens
`LD_PRELOAD=/tmp/evil.so ls` and `RUSTC_WRAPPER=/tmp/evil cargo build` now deny. Java, Go, Lua, Julia
and R were probed on real toolchains. Design record, every probe and the per-ecosystem measurements
are in **docs/design/env-prefix-classification.md**.

The last gap — `simple_verdict` propagated only `Denied` from an assignment and discarded its LEVEL,
so `RUSTFLAGS='-Cincremental=./x' echo hi` passed at `paranoid` while the identical write spelled
`touch ./x` did not — was fixed 2026-07-26 by combining the env verdict into `sub_v`. An env-spelled
effect now classifies exactly as the command-spelled one does.

## HIGH: `dispatch_wrapper` skips valued flags without looking at their values

`dispatch_wrapper` consumes a valued flag with `i += 2` and never inspects the value. That is the
same shape as the `env` handler bug (which walked past `NAME=VALUE` to reach the command) and the
`-Cincremental` bug (which matched a flag name and ignored its path). Confirmed live, all
auto-approved before this was written:

    restic --password-command /tmp/evil snapshots     runs an arbitrary program        [FIXED]
    helmfile --helm-binary /tmp/evil list             runs an arbitrary binary         [FIXED]
    vite -c /tmp/evil.js build                        loads an arbitrary JS config     [FIXED]
    sandbox-exec -f /tmp/evil.sb ls                   caller-chosen sandbox profile    [FIXED]
    dotenv -f /tmp/evil.env ls                        arbitrary env injection          [FIXED]
    borg --rsh /tmp/evil check repo                   runs it to reach the repo        [FIXED]
    borg --remote-path /tmp/evil list repo            borg executable on the far side  [FIXED]

All seven now declare a `[command.path_gate]` with role `exec`, which withholds `/tmp` and home
where `read`/`write` would admit them, while keeping the in-workspace spelling
(`vite -c ./vite.config.js`, `borg --rsh ./bin/myssh`) working. The mechanism needed no extension —
`Role::Exec` on a `[command.path_gate]` was already the right tool.

CORRECTION (2026-07-29): this section previously recorded "`borg --rsh /tmp/evil` denies, so this is
not universal". That was WRONG, and instructively so — the probe behind it omitted borg's required
repository positional, so the refusal came from the missing argument, not from any gate. With the
positional supplied, `borg --rsh /tmp/evil check repo` auto-approved and ran `/tmp/evil`. A deny
observed without checking that the command otherwise WORKS proves nothing; the guards added for this
class assert both directions for that reason.

The same section cited `nix-env -f /tmp/evil.nix` as a second example of "already denies". It does
deny — but so does a bare `nix-env -q`, because nix-env is not auto-approved in any form, so the
refusal says nothing about whether `-f` is gated. Both halves of that sentence were evidence of
nothing. Whether nix-env's `-f` needs a gate is still OPEN and becomes a real question the moment
any nix-env invocation is allowed.

**Shape of the work**, mirroring the env-prefix project: go through every `[command.wrapper]` (57 of
them) and classify each entry in `valued` as inert, a path (with a role), or a command. Inert stays
listed; a path gets a locus gate; a command recurses through `command_verdict`; anything unresearched
comes off the list and falls to approval. `restic --password-command` and `borg --rsh` are the
`RUSTC_WRAPPER` shape and should recurse. Needs a mechanism on the wrapper spec, since `valued` is
today just a list of names.

SECOND PASS (2026-07-29): the guard written for the first pass walked only `[command.wrapper]` —
307 flags — while `valued` also appears at top level (8,491), on each sub (5,825) and on a fallback
(45). Seven more executor flags were live outside it, all confirmed auto-approving with a working
baseline invocation:

    rsync --rsh /tmp/evil ./src/ ./dst/          the remote shell rsync executes
    rsync -e /tmp/evil ./src/ ./dst/             same flag, short spelling
    gotestsum --raw-command /tmp/evil            replaces the test command
    gotestsum --post-run-command /tmp/evil       run after the test run
    mypy --python-executable /tmp/evil ./src     mypy runs it to inspect the env
    pip-sync --python-executable /tmp/evil       invoked to perform the install
    kustomize build --helm-command /tmp/evil ./k the helm executable it shells out to
    steep check --steep-command /tmp/evil        the steep executable re-invoked

All gated with role `exec`; `rsync -e ssh` (a bare name on $PATH) still approves, which is the form
that actually matters. The guard now walks all four locations.

SIXTH PASS (2026-07-29) — measured the surface; NO new defects. Recorded so nobody re-runs it.

Scale of the gate surface: 1,604 commands, 14,669 valued-flag slots. 126 commands (7.9%) declare a
path_gate; 343 flag slots (2.34%) are gated. So 14,326 flag values are ungated — but that number is
NOT the exposure, and reading it as such would send the campaign in the wrong direction.

WHY it is not the exposure. An unresearched command is capped at SafeWrite: local, no execution, no
remote. A tool that merely READS an arbitrary path stays inside that cap, whatever the path — which
is why `detekt ~/.ssh/id_rsa` and `journalctl --file ~/.ssh/id_rsa` approve and are not bugs. Note
this is NOT a flag-vs-positional gap: the positional spelling approves too. Those commands simply
have no path model, and do not need one.

The bugs found in passes 1-5 were all ESCAPES from that cap — a flag value that gets EXECUTED. That
is the predicate worth sweeping, not "ungated path".

Swept clean this pass, both negative results worth keeping:
  - Reader flags. 134 flags named --input/--file/--cert/--identity/... that take an ungated path.
    Triaged: almost all are format names (`numfmt --from`), booleans (`terraform --input`), device
    specs (`findmnt --source`) or intended use (`age --identity` reading an SSH key IS its purpose).
    Reading a file into a parser is not disclosure; only content flowing OUT is.
  - Content-to-model disclosure. jq -f, xargs -a, base64, xxd, column, expand, fold, nl, rev, tac,
    paste, pr against ~/.ssh/id_rsa — every one denies, as does the `cat` control.
  - Exfil (local secret to a remote). curl -T / -d @ / -F / --upload-file, scp, rsync remote,
    aws s3 cp, gh release upload, http POST @, wget --post-file — all eleven deny.

So the remaining campaign is bounded: find flag values that reach an EXECUTION sink. The two tags
(`twin_flag`/`twin_base`, `CONFIG_IS_CODE`) cover the members already known. What neither does is
DISCOVER new members — that is still the open mechanism, and the highest-value one left.

FIFTH PASS (2026-07-29) — a TAG for the config-is-code class, plus one more finding.

`marp --config-file` / `-c` (marp.config.js is JavaScript Node executes) and `--engine` (a JS module
marp loads) were ungated; both now carry `exec`. Found not by name but by a registry-internal
differential: flags declared `standalone` in one command while `valued` in many others.

NEW TAG — `CONFIG_IS_CODE` in `a_config_flag_on_a_code_config_tool`. The fact that a TOOL executes
its config is declared once, per tool, and the guard derives the obligation for every
config-selecting flag on it (`-c`, `--config`, `--config-file`, `--noxfile`, `--conf-dir`,
`--engine`, `--format`, `--formatter`). Adding a config flag to a listed tool now FAILS until it is
gated, instead of waiting for someone to remember that tool. Currently covers webpack, vite, eslint,
stylelint, marp, nox, sphinx-build, mkdocs. Extend the list as tools are researched — jest, vitest
and cmake are gated but not yet listed; rollup, babel, gulp, grunt, playwright, cypress, storybook,
tailwind, commitlint and prettier are config-is-code but not currently auto-approved at all, so they
carry no exposure until one of them is allowed.

OPEN QUESTION (not a defect count) — 234 scopes declare the SAME flag in both `standalone` and
`valued`. SAMPLE.toml defines `standalone` as "flags that take no value", so the two declarations
contradict each other and only one can be honoured; marp was one of them. But many entries look like
deliberate modelling of an OPTIONAL value (`zstd --long` vs `--long=27`, `7z -r` vs `-r-`), which the
schema has no way to express. Before treating any of these as bugs, decide what the schema means
here: either support optional-value flags explicitly, or make the overlap a build error. A guard
written against the current ambiguity would encode a convention nobody has chosen.

FOURTH PASS (2026-07-29) — the gates were bypassable by RESPELLING. Every executor flag gated in
the passes above had an environment twin that was not gated, so the flag gate read as closed while
the same operation sailed through:

    BORG_RSH=/tmp/evil borg check repo                    (--rsh was gated)
    BORG_REMOTE_PATH=/tmp/evil borg list repo             (--remote-path was gated)
    RESTIC_PASSWORD_COMMAND=/tmp/evil restic snapshots     (--password-command was gated)
    RSYNC_RSH=/tmp/evil rsync ./src/ ./dst/               (--rsh/-e was gated)
    KUBECONFIG=/tmp/evil.yaml kubectl get pods            (a kubeconfig can carry users[].user.exec)

All now classified in envvars.toml with `shape = "exec-path"` — the LOCUS-based shape, not
`command`, so the two spellings agree: a bare `ssh` stays trusted, `/tmp/evil` does not. The classic
env-exec vectors (NODE_OPTIONS, PYTHONSTARTUP, PERL5OPT, BASH_ENV, LESSOPEN, PAGER, GIT_SSH_COMMAND)
were already covered; the gap was only the twins of tool-specific flags.

NEW MECHANISM — `twin_flag` / `twin_base` on an envvars entry, naming the flag spelling of the same
thing. `a_tagged_env_var_classifies_the_same_as_its_flag_twin` holds both spellings to the same
verdict across a foreign path, a workspace path and a bare name, and refuses a pair that never
discriminates. It fails in BOTH directions, which is how it caught kubectl over-denying relative to
its twin. Tag every new executor flag that has an env form.

REMAINING in this class: the tag only checks pairs someone DECLARED. Nothing discovers an undeclared
pair — the four above were found by hand, by asking "what is the env spelling of this flag?" for
each flag gated. A generator that proposes candidate env names per gated flag (TOOL_FLAG, FLAG) and
reports unlisted ones would turn that into a sweep. Also unmodelled: `kubectl --kubeconfig` is not a
known flag at all, so it denies as unknown rather than by gate — an over-deny to fix when kubectl's
flag surface is next researched.

THIRD PASS (2026-07-29) — the part names cannot find, partly closed. The predicate that works for
this half is the TOOL, not the flag: a build/task runner whose config file is CODE. Eight more were
live, all confirmed with a working baseline:

    webpack -c /tmp/evil.js            webpack.config.js is JavaScript webpack evaluates
    webpack --config /tmp/evil.js
    eslint -c /tmp/evil.js ./src       eslint.config.js is JavaScript
    eslint --config /tmp/evil.js ./src
    stylelint --config /tmp/evil.js    stylelint.config.js is JavaScript
    nox -f /tmp/evil.py                a noxfile is Python nox imports and runs
    sphinx-build -c /tmp/evil ./d ./o  the directory holding conf.py, executed as Python
    mkdocs build -f /tmp/evil.yml      mkdocs.yml can declare `hooks:` Python modules

All gated `exec`; the in-workspace and bare-name spellings still approve (`eslint -f json`,
`stylelint -f string`). jest, vitest and cmake were already gated. make/just/ninja/rake/rollup/
prettier/cypress/storybook/invoke/fab are not auto-approved at all, so they carry no exposure today
— but each becomes a live question the moment any invocation of it is allowed.

DELIBERATELY NOT GATED, having checked: `swc --config-file` (.swcrc is JSON swc never executes),
`esbuild --tsconfig` (JSON, not executed), `mysqldump --init-command` (SQL run by the server).

A SEPARATE BUG CLASS surfaced here and is NOT swept: webpack's `-c` was listed in `standalone`
though it takes a value, so `webpack -c /tmp/evil.js` parsed the path as a POSITIONAL and escaped
flag gating entirely. A valued flag mismodelled as a boolean defeats every flag-level gate we have.
Nothing enumerates that mismatch today; it needs a pass of its own.

STILL OPEN — the part names cannot find. Every flag above advertised itself (`-rsh`, `-command`,
`-executable`). A flag whose name hides what it does is invisible to the ratchet: `vite --config`
evaluates JavaScript and `sandbox-exec -f` picks the sandbox profile, and both were found only
because they were already written down here. Candidates seen but NOT researched: `alembic --config`
(the ini selects an `env.py` that runs), `i18n-tasks --config` (ERB-evaluated YAML), `helm/flux
--kubeconfig` (a kubeconfig can carry a `users[].user.exec` credential plugin), `workon --config`.
Each needs the per-flag research this section describes — the ~14,600 valued flags cannot be swept
by name alone.

Found 2026-07-27 while reviewing the env-assignment work.

## Follow-up: remaining JVM code-supplying flags (deliberately denied)

`-cp`/`-classpath`/`--class-path` are DONE — gated at the executor locus on `JDK_JAVA_OPTIONS`, so
they agree with the `CLASSPATH` entry. Measured: they are launcher options, accepted only there;
`JAVA_TOOL_OPTIONS='-cp …'` is `Unrecognized option` and the JVM refuses to start.

Still denied, each on purpose rather than by omission:

- `-Xbootclasspath/a:<path>` — no environment twin admits it, so there is no inconsistency driving
  it, and the bootstrap loader is a higher-privilege position than the app classpath. Adding it
  would be widening the allowlist speculatively. It is also colon-joined, and `Sep` carries only
  `equals` and `space` today, so adding the flag means adding the variant (and a test) with it.
- `--module-path=<path>` / `--upgrade-module-path=<path>` — same: no env twin, so nothing to
  reconcile. `sep = "equals"` already exists if a real need turns up.
- `-XX:VMOptionsFile=<path>` — stays denied WHATEVER the path. It injects arbitrary VM options, so a
  worktree file could carry `-javaagent`; that is config injection, the way `GIT_CONFIG_*` is, not a
  path to gate.
- `-javaagent:` / `-agentpath:` — instrument every class before main. Arguably worktree-own code by
  the same argument as `-cp`, but the capability is broader and nothing forces the question yet.

## Eleven facet axes have no authored level constraint

Surfaced 2026-07-25 by `a_declared_hazard_is_the_term_authored_levels_reject`, which verifies each
axis's declared `hazard` against the levels that actually reject it. It can only check an axis some
authored clause constrains, and it reports the ones it cannot:

    isolation, persistence.trigger.escape, supply_chain.pinning, locus.binding,
    persistence.trigger.kind, disclosure.channel, disclosure.principal,
    secret.channel, secret.principal, supply_chain.source, supply_chain.exec_surface

Two consequences worth separating.

**The hazard declarations for these axes rest on their doc comments alone.** That includes both trust
ladders — `isolation` and `supply_chain.pinning` — whose direction is inverted (higher is safer, so
the hazard is the FLOOR). `Pinning`'s doc says a level "floors it (`>= version`)"; no level does,
and per the next paragraph none is expected to. Mis-declaring `Pinning::hazard = digest` (the SAFEST
term on that ladder) would go unnoticed by that test — verified by red demo; only
`the_sentinel_is_denied_even_with_any_one_axis_relaxed` would still hold, and only because the other
axes carry the denial.

**The whole supply-chain group is unconstrained BY DESIGN, and will likely stay that way.** The
developer install clause (`levels/default.toml`, the `npm ci --ignore-scripts` shape) has landed, and
it deliberately does NOT use the supply-chain facets: it models a pinned, scripts-off install as
`execution <= self` / `persistence = installing` / `network = fetches`, because a clause admitting
`execution = network-sourced` cannot be expressed cleanly — a `<=` ceiling loosens unguarded
`ambient-config` (Makefiles and hooks slip into developer) and an exact/floor bound breaks facet
monotonicity. The pinned/scripts-off distinction is enforced at the RESOLVER, which emits the safe
shape only for that exact form; anything less emits `supply-chain-build`, which has no home below
yolo.

So `supply_source`/`pinning`/`exec_surface` are not awaiting authoring — nothing is expected to
constrain them, and their hazard declarations rest on doc comments permanently unless the level model
changes. That makes them structurally unverifiable by this test rather than temporarily so, which is
the more useful thing to know.

Not a correctness bug today: `Capability::worst()` is denied by every level below yolo, and that is
now over-determined rather than resting on `locus.local` alone.

## Loopback destinations — remaining work

Shipped 2026-07-25: `netloc::is_loopback` recognizes a local destination, `loopback_valued` gates a
flag on naming one, and `loopback_localizes` clears the destination-determined facets (remote reach,
net direction, payload, metered cost) when it does — the facets describing the OPERATION are left
alone, so the level algebra composes rather than needing a local twin of every remote archetype.
Applied to `aws dynamodb`. Decision was **per-service research, not a uniform rollout** — the
mechanism is generic but each service has to earn it.

- **Services with a real local-emulator story, unresearched.** LocalStack fronts most of AWS on
  `http://localhost:4566`; `s3api`, `sqs`, `sns`, `lambda`, `logs`, `ssm` are the common ones. Each
  needs its write surface enumerated the way dynamodb's was — the gate alone does nothing for an
  operation that isn't in the registry.
- **Other tools with an endpoint flag.** `docker --host tcp://127.0.0.1:2375` and `kubectl --server
  https://127.0.0.1:6443` were identified as candidates; neither is researched.
- **Spellings deliberately unrecognized.** `http://2130706433`, `0x7f000001`, `0177.0.0.1`, `127.1`
  are loopback in fact and denied on principle (ambiguous parsing). Revisit only if a real workflow
  needs one.
- **The tunnel caveat is structural, not a bug.** `ssh -L 8000:<service>.amazonaws.com:443` makes
  `localhost:8000` production and no static classifier can see it. This is why destroy archetypes
  may not set `loopback_localizes`, enforced at build time against the archetype's
  `operation` facet rather than its name.

## Retire blanket flag tolerance (`tolerate_unknown_short/long`)

Decision (2026-07-25): eventually remove the "any flag is fine" escape hatch. A sub that declares it
accepts flags nobody researched, which is the same unresearched-assertion problem we just removed
from the per-sub lists — only bigger, and it silently defeats the per-sub flag enforcement (a sub
with `tolerate_unknown_long = true` accepts everything regardless of what it enumerated).

Current exposure: **2,328** `tolerate_unknown_short = true` and **1,630** `tolerate_unknown_long =
true`. Concentrated in the big cloud CLIs — az (700), gcloud (389), aws (247), oci (61) — plus jj
(53), claude (37), notion (18), codex (17).

Known live consequence: `systemctl status nginx -H remote.example.com` auto-approves. `-H` retargets
systemctl to a REMOTE host over SSH, turning a local read into an operation on another system;
`status` is a legacy sub whose author-declared `tolerate_unknown_short = true` lets `-H` through.

Two distinct shapes, both bypassing per-sub enforcement:
1. **A profiled sub that declares tolerance** — enumerates flags, then accepts anything anyway.
   Still open.
2. **A `first_arg` GLOB family** — `aws s3api` matches `get-*`/`head-*`/`list-*`, so those actions are
   never profiled subs at all and never reach the flag check. **Mechanism fixed 2026-07-25; migration
   in progress — see below.**

Why it is deferred, not skipped: retiring it means enumerating thousands of per-service flags, and
until a command is done its invocations start denying. Needs planning and batching like the
re-research campaign — not a cleanup. The build already REFUSES a profiled sub that declares neither
a flag list nor an explicit tolerance, so new subs cannot quietly join this pile.

### Glob-family flag migration (shape 2) — in progress

Decision (2026-07-25): keep the verb glob, gate its flags. The `describe-*`/`get-*`/`list-*` claim is
a real, durable statement about the CLI's verb convention — it keeps covering read APIs the provider
ships tomorrow, which a hand-enumerated operation list does not. What the glob lacked was a flag
list, so it decided on the first positional and never examined the rest of the line.

`first_arg_standalone` / `first_arg_valued` (see SAMPLE.toml) now gate an admitted verb's flags.
An UNDECLARED family stays permissive — ~250 service groups can't be researched at once and denying
them wholesale would break every cloud read — and `no_new_unresearched_first_arg_family` ratchets the
remaining set so it can only shrink.

**Remaining: 237** (was 250). Migrate high-blast-radius services first; the ratchet count in that
test is the running total.

Done: aws iam, s3api, logs, cloudtrail, ec2, rds, lambda, ecs, ssm, dynamodb, cloudformation, sns,
sqs. Converted to explicit sub-subs instead (more precise, worth it for credential stores): aws
secretsmanager, aws kms, gcloud secrets.

Next by blast radius: aws eks, ecr, apigateway, route53, organizations, sts-adjacent identity
services; then az (~700 tolerances) and gcloud (~389), which also still need the structural
subgroup-glob fix tracked in the cloud-CLI notes.

The per-service payoff is in the flags each service withholds, which is why this can't be done
generically. Real examples found so far: `logs --unmask` (returns data-protection-masked log content
in the clear), `ssm --with-decryption` (decrypts SecureString values), `s3api --sse-customer-*`
(supplies caller-held key material). Every family also withholds `--endpoint-url`, `--profile`,
`--ca-bundle`, `--no-verify-ssl`, `--no-sign-request`.

## Two new fuzz targets — both bugs FIXED, both wired into nightly

Added and enabled 2026-07-30. `equivalence` (semantics-preserving respelling cannot change the
verdict) and `hook_envelope` (a target never emits a grant it was not asked for). Both ran red on
day one against real bugs; both are green now and run nightly from the `property-targets` job, each
with its own corpus so they accumulate coverage independently.

FIXED — the two gates are now ONE rule. `path_gate` role `exec` and envvars `shape = "exec-path"`
were separate implementations and had drifted: the env side always split a value on `:`, the flag
side never did, so `BORG_RSH=x:/tmp/evil` denied while `borg --rsh x:/tmp/evil` was approved.
`engine::resolve::worst_path_element` is now the single rule both call.

The interesting part was the DIRECTION. Making the flag side split too broke every `curl` test in
the suite, because a URL is not a path list (`https://example.com` split to `https` +
`//example.com`). The two sides differ because the VALUE TYPES differ, not because the rule did: a
search path is a list, `BORG_RSH` is a command. So list-ness is now explicit — splitting is the
default and `single_value = true` is the opt-out — and the direction matters: a real list judged
whole is a FAIL-OPEN, while a single value split is merely stricter.

FIXED — the blank-command rule moved to `targets::respond`, the shared decision seam. It was in
`main.rs`, so the shipped binary was safe while `render_response` (public, and the owner of the
decision contract) knew nothing about blankness. Any second caller reintroduced it, and the
integration guard passed only because it drives the binary.

Worth keeping in mind: the hand-written twin guard tried three values and passed. The fuzzer needed
a fourth (`:/:`). The guard's corpus is now thirteen values spanning colons, traversal, roots and
bare names, and `worst_path_element_splits_only_a_list_and_never_a_url` pins the rule directly,
including that splitting can only ever be STRICTER than judging whole.

## Fuzzing finds availability bugs only — two targets worth adding

State (2026-07-30): healthy and quiet. Nightly green five nights running (~5h, sharded), replay
green on every push, no crash/timeout/oom artifacts, corpus merged to 18,013 inputs.

But `fuzz_targets/parse.rs` ends in `let _ = is_safe_command(&command);` — the verdict is
DISCARDED by design, so the only contract under test is "does not panic or hang". Measured against
that: every one of the ~30 defects found in the 2026-07-29/30 review session was invisible to it.
The heredoc-body fail-open, the 24 executor-flag gaps and the env-twin bypasses are all
CLASSIFICATION bugs — the fuzzer runs them and sees no panic. The hook blank-command approval, the
`--setup` panic and the `--suggest` phishing vector live in `targets/*` and `suggest.rs`, which
`is_safe_command` never calls.

That is not a criticism of the target: it does its job, and the availability contract it guards is
real (a panicking PreToolUse hook fails OPEN). It just means the fuzzing budget currently buys
nothing against the bug class that actually dominates.

1. **A METAMORPHIC target — the highest-value one.** Assert a property OF the verdict instead of
   discarding it: a semantics-preserving respelling must not change the verdict. Every class found
   this session was exactly a respelling gap — `--flag=V` vs `--flag V` vs `-fV`, an env twin vs its
   flag, a heredoc body vs a herestring. Generate a command, apply a transform that provably does
   not change what the shell does, assert the two verdicts are equal. Unlike the current target this
   can find fail-opens, and it needs no oracle beyond self-consistency. Start from the transforms
   already hand-written as guards (`FormCase` flag-form equivalence, the twin tag, the heredoc
   herestring equivalence) — they are the seed set.

2. **A hook-envelope target** for `targets/*` (already noted in AGENTS.md §Fuzzing). Feed arbitrary
   bytes as an envelope to each target's `parse_input` + render path. The blank-command approval and
   the wrong-typed-key panic were both found by hand there; a target would have found both and keeps
   finding them as harnesses change. Note a subtlety: the interesting contract is not only
   "no panic" but "never emits an ALLOW decision it was not asked for", which is checkable.

Neither needs new machinery — `cargo fuzz` is already wired, sharded and merging corpora nightly.

## THE campaign — re-research every command (see RESEARCH-PLAN.md)

Decision (2026-07-16): re-research and upgrade the TOML of EVERY command under the facet model. No
shortcuts — the level-based tail hid real credential exposures (`vault read`, `security
find-internet-password`, `aws secretsmanager get-secret-value`). Batched, highest-risk-first, with a
targeted adversarial review after each batch and a general facet-vocab assessment every ~2–3 batches.
Full plan, standard, batch order, and cadence in **RESEARCH-PLAN.md**. Next: **Batch 0 (credential
slice)** — classify the 17 subs on the `credential_smelling_subs_*` guard's grandfather worklist.

## Pre-1.0 hardening

- **Credential-exposure audit — the #1 correctness item (the one class that escapes the SafeWrite-local
  bound: a "read" that returns REMOTE secret material). SWEEP SPEC BUILT + partly gated.**
  - Two guards enforce the class: `credential_smelling_subs_are_classified_or_grandfathered` (sub NAME
    layer) and the new `credential_store_reads_are_denied` corpus ratchet (ARGUMENT / whole-tool layer).
    IMPORTANT: the class CANNOT be swept generatively — a blind `<read-verb> <secret-word>` probe is
    vacuous (1855 false hits: `alembic show secret` auto-approves because `show` takes any positional).
    So the ratchet is a curated researched worklist that only grows as secret-store CLIs are researched.
  - Gated this pass: `op item get`/`read`/`document get` (profile=credential-read; op is a whole secret
    store), `vault kv get` (the KV-v2 sugar for `vault read`). Regression-covered: aws secretsmanager
    get-secret-value / ecr get-login-password / sts get-session-token / ssm get-parameter
    --with-decryption, gcloud secrets versions access / auth print-*-token, az keyvault secret show, gh
    auth token, doctl auth init, security find-internet-password.
  - kubectl `get secret` — GATED (2026-07) via the new `credential_first_arg` mechanism (below): every
    name form denies — exact `secret`/`secrets`, the slash shorthand `secret/<name>`, qualified
    `secret.v1.core`, and flag-first `get -o yaml secret` — while pods/CRDs/`secretstore` stay read-only.
    Residual (minor): conservative — gates `get secrets` (name list) too; `describe secret` stays allowed
    (it redacts values).
  - NEW MECHANISM `credential_first_arg` (2026-07) — the value-dependent credential gate. A glob list on
    a Branching sub (dispatch_branching, flag-aware) that DENIES a first-positional match before the
    first_arg allow-glob. The declarative complement to `profile=credential-read` for the "a specific
    resource/key name discloses" class. Closes kubectl secret (all forms) AND `aws configure get
    aws_secret_access_key`/`aws_session_token` (region/output stay allowed). Guarded by
    `credential_first_arg_gates_every_secret_name_form`; documented in SAMPLE.toml.
  - Breadth sweep batch 1 (2026-07): gated `bw get`/`list` (Bitwarden), `pass show`/`grep`, `heroku
    config` (all profile=credential-read; conservative on the password managers). Verified already-safe:
    doppler, gopass, chamber, infisical, `az account get-access-token`, `gcloud auth print-*`, flyctl,
    step, kubeseal, gpg -d, `cat ~/.aws/credentials`. `wrangler secret list` = names-only (grandfathered).
  - DECRYPT-TO-SCREEN — DONE (2026-07). New `decrypt-read` archetype (operation=observe, secret=reads,
    disclosure=local-process → yolo, the same tier as a credential-store read) + a NEW top-level
    `[[command.flag]]` mechanism (the flat-command analog of `[[command.sub.flag]]`: a mode flag whose
    presence classifies the whole invocation as an archetype). Closed:
    - `sops` — restructured to 3.13 subcommands. `decrypt` sub + legacy `-d`/`--decrypt` flags →
      decrypt-read; `filestatus` → SafeRead; `encrypt`/`edit`/`rotate`/`set`/`exec-env`/`exec-file`/
      `updatekeys` → candidate (remote KMS / interactive / decrypt+execute). Closed BOTH the `-d` flag
      hole AND the newly-found subcommand hole (`sops decrypt FILE` was read as a filename → SafeWrite).
    - `age -d`/`--decrypt` → decrypt-read (encrypt stays SafeWrite).
    - `ansible-vault view` AND `decrypt` → decrypt-read (`decrypt --output -` streamed plaintext to the
      model — a bypass caught by adversarial review; `view` was gated but `decrypt` was left SafeWrite).
    - `gpg -d`/`--decrypt` → decrypt-read (top-level flag). Also `gpg secret.gpg` (bare-file IMPLICIT
      decrypt) now denies: gpg requires an inspection command (`require_any`), so a positional-only
      invocation can't auto-approve an implicit decrypt/verify.
    - `openssl` (all disclosure subs: rsa/pkey/ec/dsa/pkcs8/pkcs12/enc/smime/cms) → an ENGINE RESOLVER
      `resolve_openssl` (src/engine/resolve.rs), NOT declarative. openssl's flag grammar (single-dash
      long opts `--d`==`-d`, the `-text` side channel that dumps private components past `-pubout`/
      `-noout`, an `-out` whose VALUE can be stdout, parser token-swallow) defeated declarative
      matching over 3 review rounds. The resolver emits decrypt-read (→ yolo) only when private/
      decrypted material reaches the MODEL (stdout), and abstains for public-key mode, to-FILE
      extraction, `-noout` validate, and encrypt/sign. `openssl_output_reaches_model` is FAIL-CLOSED
      (model-reaching unless a single plain-file `-out`). The superseded declarative scaffolding
      (`unless_flags`, bimodal-sub walk, single-dash-long flag normalization) was REMOVED. A glued
      `-flag=path` pathgate gap (out-of-workspace read via `-in=~/.ssh/id_rsa`) was fixed generally in
      `pathgate::walk`. Key GENERATION (genrsa/genpkey/req -newkey) stays SafeWrite — the threat model
      is exfil of EXISTING secrets, not fresh keys (user-confirmed).
    Guarded by `decrypt_read_denies_at_the_band_and_is_a_secret_read` (registry-walking) +
    `decrypt_to_screen_corpus_denies` (MUST_DENY corpus + a complement of diverted/read forms that must
    stay allowed) + `openssl_output_destination_is_fail_closed` + `openssl_resolver_gates_model_
    disclosure_only` (golden) + `openssl_decrypt_triggers_gate_both_dash_spellings`. The user's rule:
    decrypt-to-screen is NOT auto-approved below local-admin (lands at yolo, refused below). CONVERGED
    after 7 adversarial-review rounds (the last comprehensive pass: clean, fail-closed by construction).
    - `aws configure get aws_secret_access_key` / `aws_session_token` — GATED (2026-07) via
      `credential_first_arg` on `configure get`; `get region`/`output` stay allowed. (Residual: the rare
      profile-qualified key form `get profile.x.aws_secret_access_key` needs suffix-glob support — the
      current globs are prefix-only.)
    - `terraform output -raw <name>` and `helm get values <release>` — VALUE-dependent: mostly non-secret
      outputs/config, but a sensitive output / a secret embedded in values discloses. Handler-class (can't
      gate the whole sub without over-denying the common read). Grouped with the value-dependent set.
  - INTENTIONALLY ALLOWED (verified, not holes): `kubectl get configmap -o yaml` (ConfigMaps are officially
    non-secret; gating over-denies config reads); `cat .env` (worktree-local — the workspace-boundary model
    lets the agent read its own project files; a remote EXFIL of it still denies).
  - Remaining sweep: keep researching secret-store / cloud CLIs and add each credential read to the ratchet.
    The value-dependent class (sops -d, aws/terraform/helm/kubectl-configmap) wants a shared "flag/first-arg
    triggers credential-read" mechanism — worth designing once rather than per-tool handlers.
- **Over-deny audit follow-ups — RESOLVED (2026-07).**
  - `terraform`: already fully covered (verified) — `plan`/`validate`/`show`/`fmt`/`output`/`state list`
    /`version` allow, `init`/`apply`/`destroy`/`import` deny. The old "not covered at all" note was stale.
  - `fd -x`/`--exec` / `-X`/`--exec-batch`: NOW delegates to the inner command like `find -exec`
    (`handler = "fd"`, `src/handlers/fd.rs`), bound to each search path (deny-absorbing); the no-`{}`
    batch form appends the match so `fd /etc -X cat` can't leak. Proptest `fd_exec_follows_the_inner_
    command_locus` guards the class.
  - Judgment calls MADE (keep denying — opaque/network-sourced code, the `./bill` line): `pnpm install`
    (postinstall) and `python3 -m <module>` deny. `npm run` already allowlists safe scripts via
    `first_arg` (`run test` allows, `run build` denies) — no change needed.
- **Harness verification grid — see the scorecard at the top of HARNESS-BEHAVIORS.md (source of truth).**
  Verified live: Codex, Antigravity `agy` (supersedes retired Gemini), Claude, Cursor, Copilot
  (v1.0.71, allow+deny both honored). Assumed (Claude-mirror): Qwen, Droid. opencode is static-config
  (no runtime hook).
- **Cursor target — DECIDED (2026-07): Deny harness.** cursor-agent v2026.07.16 honors a hook `deny`
  (blocks + shows our message) but IGNORES `allow` (a known cursor bug — forum.cursor.com/t/…/144244,
  allowlist wins). So `src/targets/cursor.rs` now emits `deny` for gated commands (protective, like
  Codex) and keeps `allow` for safe (inert until cursor honors it). REVISIT if the bug is fixed → switch
  back to allow-for-safe + Defer. Trade-off: a Deny harness hard-blocks every not-allowlisted command
  (escape = `~/.config/safe-chains.toml` grant), stricter than the prior abstain. See HARNESS-BEHAVIORS
  §Cursor.
- **opencode — DROPPED `--opencode-config` (decided 2026-07).** It rendered an empty allowlist (the
  `all_opencode_patterns()` stub) — misleading — so the flag, stub, and renderer were removed;
  `OpenCodeTarget` stays for detection with a "no usable hook yet" message. opencode has no runtime hook
  (plugin hook broken, opencode #7006) and a static glob can't express per-arg safety, so there is no
  meaningful integration to ship. **WATCH-LIST (revisit when upstream changes):** opencode #7006 (a real
  runtime hook) → then wire a per-command opencode target. Also Cursor forum 144244 (the ignored-`allow`
  bug) → when fixed, flip `targets/cursor.rs` back to allow-for-safe + Defer (see §Cursor).
- **`.safe-chains.toml` protected config location — WON'T-FIX before 1.0 (decided 2026-07).** Most
  harnesses do not expose a protected location, so there's nothing to implement. Best-effort holds: the
  command classifier denies every *command* write to the trust root (guarded); a non-command write
  (editor/`python -c`) escaping it is an accepted residual, out of scope for a string classifier.
- **cargo-fuzz — DONE (2026-07).** `fuzz/` standalone-workspace crate, `parse` target over
  `is_safe_command`, seed corpus, nightly Linux CI (`.github/workflows/fuzz.yml`). Verified live:
  builds under nightly + cargo-fuzz 0.13.2, 416k runs/26s clean. Run with `cargo +nightly fuzz run
  parse`. Follow-ups: pin a dated nightly for CI reproducibility; add a `command_verdict` target.

---

## Post-1.0 (deferred, not blocking 1.0)

- **ANSI-C quoting (`$'…'`) is unmodeled, so every use of it denies.** Found 2026-07-28 by a
  differential sweep against `bash -n`. The parser has no `$'…'` token: simple cases survive by
  accident (the `$` is dropped and `'…'` parses as an ordinary single-quoted string), but the
  quoting rules differ, so `echo $'don\'t'` fails to parse outright — in `$'…'` a `\'` is an
  ESCAPED quote, while in `'…'` a `'` always closes. Uniformly fail-closed today: even
  `cat $'README.md'` denies, so nothing is mis-approved and there is no hurry.

  The cost is over-denial of real idioms — `sort -t$'\t'`, `IFS=$'\n'`, `grep $'\t'`.

  Deliberately NOT fixed inline with the heredoc work, because decoding `$'…'` means decoding
  escapes (`\x2f`, `\057`, `\n`), and that is a new PATH-NORMALIZATION surface: `cat $'\x2f\x65\x74\x63/shadow'`
  must classify as `/etc/shadow`, not as an opaque literal. Adding the token without the decoding
  would turn today's uniform deny into a hole. Verified current behavior is safe: the hex and octal
  spellings of `/etc/shadow` and of `../outside.txt` all deny.

  **Done when:** `$'…'` is a real `WordPart` whose escapes are decoded before locus classification;
  `sort -t$'\t' file` and `echo $'don\'t'` approve; every escape spelling of an out-of-workspace
  path still denies, guarded by a property test over encoded/plain path pairs (the encoded form must
  never be more permissive than the literal one — the abstraction-soundness shape already used in
  `handler_property_tests`).

- **Em-dash sweep of command descriptions.** The hand-written guide docs (`docs/src/*.md`) and
  `README.md` are em-dash-free (done 2026-07). The generated Command Reference still carries them: 26
  em-dashes surface in `COMMANDS.md`, sourced from the `description` field of ~560 of 1257 command
  TOMLs. Sweep them context-aware (colon / comma / period / parens per usage, not a blind `sed`), then
  regenerate `COMMANDS.md` + the book. Also add a "no em-dashes in descriptions" note to the
  description-writing guidance in `AGENTS.md` so new commands don't reintroduce them. Deferred by user
  decision — not needed for 1.0.

---

## DONE (2026-07-16)

- **Pre-fanout adversarial review — 2 mechanism fail-opens found + fixed, credential class surfaced.**
  Probed the new resolvers/mechanisms for evasions a fan-out would multiply. Found + fixed: (1)
  `npm ci --ignore-scripts=false` / `--no-ignore-scripts` / `=0` auto-approved (the `when_absent`
  escalator used a loose `flag_present` that counted `=false` as set) → new `flag_is_affirmatively_set`
  (bare / `=true` set; `=false/0/no/off` / `--no-` disable). (2) `git push --repo=ext::sh origin`
  auto-approved (destination classifier only saw the positional; `--repo` overrides it to an RCE
  transport) → new `destination_flag` on the sub, classified with the same provenance rules. Both
  fixes generalize (every install's scripts flag, every push-like command's destination flag). Also
  surfaced the credential-exposure CLASS (below, the #1 fan-out item) and tagged clean exemplars
  (gcloud/vault). Clean on probes: sudo flag-parse, system-integrity path spellings (all deny at
  default), per-level chain integrity, install-clause loosening. Regression guards:
  `value_prefix_flags_escalate_only_on_a_matching_value` (+ `when_absent` cases),
  `git_push_destination_provenance_is_classified` (+ `--repo` cases).

- **Credential + remote-exec archetypes — the last fan-out vocabulary gaps.** Three static archetypes,
  all landing at yolo (recognize-and-route): `credential-read` (`secret = reads` — `gcloud auth
  print-access-token`, `gh auth token`, `aws secretsmanager get-secret-value`, `vault read`),
  `credential-mint` (`secret = writes` — `aws iam create-access-key`, `aws sts get-session-token`,
  `kubectl create token`), and `remote-exec` (`operation = execute` on a remote — `kubectl exec`,
  `ssh cmd`, `aws ecs execute-command`). They land at yolo automatically: `secret <= uses-ambient` on
  every level below yolo, and `execute` is absent from network-admin's op list (and local-admin's
  execute clause is `remote = none`). Distinct from `remote-read` (an identity read like `aws sts
  get-caller-identity` is NOT a secret) and `remote-authorize` (grants access with ambient creds, no
  new material). Proof: `archetypes_land_where_the_catalog_says`. FAN-OUT VOCABULARY now complete:
  remote-{read,create,mutate,destroy-recoverable,destroy-irreversible,authorize,control,exec},
  credential-{read,mint}, vcs-sync, {supply-chain-build, local-install-pinned}, blockchain-txn,
  {local-privileged, privileged-control}. NUANCE for fan-out guidance (not blocking): a file-TRANSFER
  sub (`aws s3 cp s3://… ./x`) is remote-read PLUS a local write — classify the local-write
  destination, don't tag it pure remote-read.

- **Flag-conditional-archetype resolver + npm exemplar — the install clause is now live on a real
  command.** New mechanism: `when_absent = true` on a `[[command.sub.flag]]` INVERTS the escalation —
  a SAFETY flag whose ABSENCE is the risk. npm's `ci` sub is `profile = "local-install-pinned"` with a
  `when_absent` flag on `--ignore-scripts` → `supply-chain-build`: `npm ci --ignore-scripts` stays
  local-install-pinned (developer), `npm ci` (scripts on) escalates to supply-chain-build (yolo). The
  floating `install`/`i` subs are static `supply-chain-build`. This CORRECTLY TIGHTENS the old
  `npm ci = SafeWrite` (which auto-approved even while running lifecycle scripts). Build guard:
  `when_absent` ⊥ `value_prefix`. Corpus-gate fix: skip profiled subs from the never-looser check
  (their legacy kind is a deny-all artifact, not a real baseline — `npm ci --ignore-scripts` is the
  first profiled sub to land IN the auto-approve band). Proofs:
  `npm_install_is_classified_by_pinning_and_scripts_off`, the `when_absent` flag_escalates case,
  npm examples_safe/denied. PATTERN for the fan-out: each manager gets pinned-sub + `when_absent`
  scripts-off flag (pip `--require-hashes`, cargo `--locked` + build.rs always-runs, yarn/pnpm
  `--frozen-lockfile --ignore-scripts`).

- **Supply-chain / install clause authored — the developer "pinned + scripts-off" install.** Closes
  the vocabulary gap that would have corrupted the package-manager slice of the fan-out. KEY DECISION:
  a scripts-OFF install does not EXECUTE foreign code — it fetches files and writes them; the code
  runs later at call-time when you run your own program (a separate command). So the safe install is
  modeled `execution = self` / `persistence = installing` / `network = fetches`, NOT a guardrail-gated
  `network-sourced`. This was forced by a real finding: a clause admitting `execution = network-sourced`
  can't be expressed cleanly — a `<=` ceiling loosens unguarded `ambient-config` (Makefiles/hooks slip
  into developer), an exact/floor breaks `authored_levels_are_facet_monotone` (the guardrails make a
  higher execution rung safer than a lower one). The `execution = self` model keeps the clause all-`<=`
  and monotone. Landed: the developer install clause (`create/mutate · <= worktree · installing ·
  fetches · execution <= self`; capped at `worktree` NOT `worktree-trusted` so `.git`/`.envrc`/hooks
  stay write-frozen — caught by 7 redirect/hot-operand tests), plus TWO archetypes completing the
  vocabulary: `local-install-pinned` (safe → developer) and the reframed `supply-chain-build`
  (scripts-on/unpinned, network-sourced → yolo). Proof: `pinned_scripts_off_install_is_developer_the_supply_chain_surface_is_yolo`
  + the catalog test. REMAINING (the fan-out-able part): a resolver that PICKS the archetype from the
  command form (`npm ci --ignore-scripts` → local-install-pinned; else → supply-chain-build) — a new
  flag-conditional-archetype mechanism + per-manager research (npm/pip/cargo/yarn/pnpm pinned-form +
  scripts-off flag).

- **Machine locus SUB-RUNGS + facet-correct systemctl (the `restart nginx` ≠ `/etc/passwd` insight).**
  Two coupled refinements, from the observation that `machine` locus "means different things depending
  on the other facets":
  - **New `LocalLocus::SystemIntegrity` rung** between `machine` and `device`: the machine's own
    identity/auth/boot/loader substrate (`/etc/passwd`, `/etc/group`, `/etc/sudoers`(.d), `/etc/pam.d`,
    `/etc/ld.so.*`, `/boot`) where a WRITE is compromise-complete. Read face stays `machine`; the WRITE
    face worst-cases to `system-integrity`. Routed via a small, deny-ward `[role.system-integrity]` in
    `regions/default.toml` (fail-closed, like the credential shield — NOT an admit map). `local-admin`
    tightened from `locus ≤ device` to `locus ≤ machine`, so ordinary machine admin (a service, an app
    config) is local-admin while owning the trust root (`/etc/passwd`, `/boot`) is yolo-only. Safe by
    construction: the auto-approve band already denies all of `machine`, so only the upper bands split.
  - **systemctl archetype split**: `restart`/`start`/`stop`/`reload`/`kill`/`daemon-reload`/… now
    `profile = "privileged-control"` (new archetype: `control · machine · root · recoverable ·
    transient` — runs existing config, no durable change), while `enable`/`disable`/`mask`/`set-default`
    stay `local-privileged` (`configure · machine · root · effortful · installing`). Fixes the factual
    mismodeling where `restart` claimed `persistence = installing` (a phantom install). Both still land
    at local-admin (authority=root is the gate vs developer — expressed correctly, as the user required).
  - Proofs: `system_integrity_is_above_local_admin_ordinary_machine_is_not`,
    `regions::…system_integrity_substrate_write_worst_cases_above_machine`,
    `archetypes_land_where_the_catalog_says` (privileged-control → local-admin).

- **systemctl service management → `local-privileged` (first real user of the archetype).** Added the
  privileged subs (start/stop/restart/reload/…/enable/disable/mask/unmask/daemon-reload/…/isolate/
  set-default/kill/reset-failed/set-property) as `profile = "local-privileged"` with per-sub
  fact+source; read subs stay SafeRead; power-state (reboot/poweroff/halt/…) and `edit` are omitted →
  deny by omission. So `systemctl restart nginx` lands at local-admin, and `sudo systemctl restart`
  (previously fail-closed — the inner sub was unmodeled) now resolves to local-admin too. network-admin
  correctly refuses (local privilege, not remote). Proof: `systemctl_service_management_is_local_admin`
  + examples_safe/denied. This validates the local-privileged archetype end to end and gives
  local-admin a real command, not just the `sudo` wrapper.

- **CLI per-level classification — the upper-level model is now reachable.** `--level
  {local-admin,network-admin,yolo}` classifies via `Level::admits` instead of the 3-value ceiling.
  A scoped eval-level context (`bridge::enter_eval_level`) is consulted in the ONE chokepoint every
  resolve path funnels through — `bridge::project` — so a profile that only an upper level admits
  (`git push`) is approved when the threshold IS that level, while the lower band
  (`paranoid`..`developer`) stays byte-for-byte unchanged (no context → the old projection).
  `main.rs` resolves the `--level` name (legacy aliases too) to `upper_level_by_name` and routes to
  `command_verdict_at_level`. Result: `git push origin` allows at network-admin/yolo, `rm -rf /`
  denies even at yolo, unmodeled/legacy-denied commands stay denied at every level (allowlist-only).
  Proofs: `upper_band_levels_admit_via_the_engine_end_to_end`,
  `cli_gate::upper_band_level_thresholds_gate_through_the_cli`.
- **`sudo`/`doas` wired — `local-admin` now has something to admit.** They are an authority-elevating
  DELEGATING WRAPPER, not a flat archetype: `resolve::resolve_privilege_wrapper` parses sudo's options
  (fail-closed on any unknown one, and on `-i`/`-s`/`-e` root shells/editors), resolves the INNER
  command, and lifts every capability's `authority` to `root` (or `other-user` for `-u`). Since every
  band below local-admin pins `authority = user`, a root op lands at local-admin: `sudo cat ./notes`
  → root read (local-admin/yolo), `sudo rm -rf /` → still the catastrophe corner (denied everywhere),
  `sudo -u bob …`/`sudo -i` → yolo-only, `sudo <unmodeled>` → fail-closed. NEVER LOOSER: at the
  default band every `sudo …` denies (root authority auto-approved by no level ≤ developer), matching
  the legacy classifier. Proof: `sudo_elevates_the_wrapped_commands_authority`. (`-u root` is
  conservatively treated as `other-user` — a fail-closed over-deny of a rare redundant form.)
- **Destination-aware push resolver — `locus.provenance` now binds to a command.** New declarative
  `network_destination = true` on a profiled sub (`registry::sub_destination_token` extracts the send
  target; `resolve::destination_provenance` classifies it): bare name → `established`, URL / scp-path
  / filesystem-path → `literal`, `$VAR` → `opaque`; `ext::<cmd>` worst-cases as RCE. Declared on
  git's `push` sub; generalizes to scp/rsync/curl -d. So at network-admin, `git push origin` and
  `git push https://host` allow while `git push $REMOTE` and `git push ext::…` deny. Build guard:
  `network_destination` requires a `profile`. Proof: `git_push_destination_provenance_is_classified`.

- **Destination-trust → new `locus.provenance` facet** (behavioral-taxonomy-exposure.md §4). Answers
  the §8 open question: it is a genuinely new axis, not a decomposition. `na < established < literal
  < opaque` — how the acted-on target was *designated*. Neither existing candidate expresses it:
  `network.destination` measures breadth (`git push origin` and `git push https://evil` are both
  `fixed`); `locus.binding` measures visibility with the OPPOSITE polarity (for push, both `origin`
  and a URL are `pinned`, while the most-trusted bare push is `ambient`). Wired: the facet + `Locus`
  field, `Clause`/authoring/archetype TOML parse, the term roundtrip/zero/ladder tests, and the
  proptest generators (`arb_capability`/`arb_clause`). `network-admin` caps `provenance <= literal`
  (a human reviewing at that level sees the URL); `opaque` (a `$VAR` target) lifts only at `yolo`.
  `ext::<cmd>` stays OUT of this facet — it's `execution` (RCE), not a destination. Red→green proof
  `a_literal_send_target_is_network_admin_an_opaque_one_is_yolo`. REMAINING (below).

- **git remote handler → declarative subs.** `check_git_remote` (+ the `GIT_REMOTE_READONLY` WordSet)
  was pure DATA — a read/mutate allowlist, no logic — so it moved into `commands/vcs/git.toml`:
  `git remote` is now a Branching sub (`nested_bare` + `-v`/`--verbose` list; `get-url`/`show` query
  at SafeRead; mutating subs add/remove/rename/set-url/set-head/prune/update deny by OMISSION). The
  `git_remote` registration is gone from `handlers/mod.rs`. Behavior identical (every `git_remote_*`
  test passes unchanged). What stays in `is_safe_git` is the one genuine piece of LOGIC: the `-c` gate.
- **git `-c` gate reframed as a positive allowlist.** `GIT_C_KV_EXACT` → `GIT_C_ALLOWED_KV`,
  `is_safe_git_c_kv` → `is_allowed_git_c`; comments describe only what is PERMITTED (exact safe
  `key=value` settings + the `color.*` / `advice.*` / `safe.directory` namespaces), and everything
  else denies by omission — no reasoning about which keys are "blocked". The code was already a
  positive allowlist; this removes the last denylist-flavored framing (naming `core.sshCommand` as a
  thing-we-deny). Behavior identical (safe configs approve, unlisted keys deny).
- **`disclosure.audience = public`: gate → record (level side).** Per behavioral-taxonomy-exposure.md
  §3/§7. `network-admin` now admits `disclosure.audience <= public`; the confidentiality danger is
  gated on CONTENT — the `secret <= uses-ambient` ceiling already on that clause — so publishing your
  own NON-secret content (git push to a public repo, `npm publish`) is a network-admin op, while
  transmitting a SECRET off-box stays yolo. Proof (red→green):
  `public_disclosure_is_recorded_not_gated_secret_transmission_is`. The *destination-trust* half
  (established vs inline vs dynamic target) is resolver-side and still open (exposure.md §4/§8).

## DONE (2026-07-14)

- **Overreach nudge now NAMES the working directory (UX).** The "reaches outside the working
  directory" message named the reached path but not the cwd, so a user who forgot which directory
  they launched the agent from couldn't spot a directory MISMATCH (a common, easy-to-forget mistake —
  e.g. cross-repo work). Now it reads "…outside the working directory `<cwd>`. If the agent is
  running from the wrong directory — an easy thing to forget — relaunch it where you meant to be…"
  (`main.rs::outside_workspace_clause`, used by the Deny/Ask clause and the Defer nudge). Guard:
  `tests/cli_gate.rs::overreach_nudge_names_the_working_directory`. (Future: actually moving the
  harness cwd would need a harness capability we don't have — surfacing the mismatch is the best we can do.)
- **CLI-gate FAIL-OPEN fixed (found by the other agent session).** A typo'd/unknown flag
  (`safe-chains "rm -rf /" --levle inert`) used to exit 0 = "allowed": clap's parse error fell
  through to hook mode, which read empty stdin and exited 0. Now the `Err` arm is `e.exit()` (clap
  prints the error, exits 2; help/version exit 0) — malformed CLI FAILS CLOSED. Every legit hook
  invocation (`safe-chains` bare, `safe-chains hook <target>`) parses cleanly and never hit that arm.
  Guarded by `tests/cli_gate.rs` (subprocess exit-code contract).
- **Absolute-path-inside-root OVER-DENY fixed (found by the other agent session).** `cat /root/proj/x`
  denied while `cat x` allowed — same file, different spelling (calling-conventions violation).
  `pathctx::resolve` returned absolute paths as-is; now it also normalizes an in-root absolute path to
  root-relative (worktree), so both spellings classify identically. Out-of-root absolutes (system,
  sibling repo, `..`-escape) still deny (the `inside.starts_with('/')` guard stops `/proj-evil`
  matching `/proj`). Guards: `pathctx::…absolute_in_root_becomes_root_relative…` +
  `absolute_and_relative_in_root_paths_classify_identically`.
- **Verified NOT a bug: hook cwd source.** The claude hook extracts cwd from the JSON PAYLOAD
  (`envelope.cwd`) and root from `CLAUDE_PROJECT_DIR` (defaulting to cwd) — NOT the process cwd.
  Confirmed via stdout (payload cwd=billlocal → a safe-chains-repo read is correctly gated).
- **Config trust-model audit + self-escalation lock.** Verified the model is SOUND: user config
  (`~/.config/safe-chains.toml`) is the root of trust; repo `.safe-chains.toml` is honored only when
  the user PINS its dir AND the SHA-256 matches (approve-then-mutate is caught); `XDG_CONFIG_HOME` is
  not honored (env-redirect closed); the trust root has an un-grantable region shield (a `~/` grant
  can't unlock its write). Added the systemic COMMAND-level guard `trust_root_is_unwritable_by_any_command`
  (13 write vectors × 4 path spellings all deny; reads stay OK) — pins end-to-end what the region test
  only checked at the locus level. Residual (documented): the config path is unprotected against a
  NON-command write (editor/python) → the "protected third-party config location" research item.
- **Over-deny fix: grep `-P` / `--perl-regexp` (PCRE).** Was worst-cased as "code-executing PCRE",
  but that `(?{code})` construct is a PERL feature NOT in PCRE2 (what GNU grep `-P` uses) — so it
  runs no code, as safe as `-E`/`-F`. Fixed in the engine `resolve_grep`: added `P` to the benign
  short set, dropped `--perl-regexp` from `grep_long_dangerous`, added it to `grep_long_known`.
  `-R`/`--dereference-recursive` stays denied (real symlink-out-of-locus concern). Tests updated
  (`grep -P`/`-oP`/`--perl-regexp` now read-local). `grep -oP '\d+'` allows.
- **Deeper fuzzing — found + fixed TWO crash/DoS bugs.** (1) `classifier_terminates_on_adversarial_input`
  (worker-thread timeout over a pathological corpus) caught a STACK OVERFLOW: the CST recursive-descent
  parser recursed per nesting level, so `"("×100000` overflowed and ABORTED the process (an unwind-proof
  crash → fail-open hook). Fixed with a `MAX_PARSE_DEPTH = 48` bound on `script()` (the single recursion
  chokepoint). (2) The same guard caught an O(n²) re-scan in perl's `strip_inert_string_text` on
  `"@{@{@{…"` — fixed to copy-rest-and-stop (O(n)), security preserved. Also broadened
  `handlers_never_panic_and_are_deterministic` to the FULL registry (~1257) and added
  `arbitrary_command_strings_never_panic` (arbitrary shell-metachar strings through the whole pipeline).
- **find denylist → allowlist (last live handler denylist).** `find.rs` was allow-all-except
  `FIND_DANGEROUS_FLAGS` (fail-open: a new/BSD write primary slipped through). Now `FIND_SAFE_STANDALONE`
  + `FIND_SAFE_VALUED` enumerate the read-only primaries (tests, `-print`/`-ls`/`-prune`, operators,
  positional/global opts, `-newer*` by prefix); unknown/write primaries deny by omission. Valued
  primaries consume their value (`-mtime -7`, `-name -delete` handled). `-exec`/`-execdir` delegation
  unchanged. **Every handler denylist is now converted — the ratchet GRANDFATHERED set is empty.**
- **perl double-quote INTERPOLATION bypass (was: RCE).** `perl -e 'print "@{[system(q(id))]}"'` was
  ALLOWED — `content_outside_double_quotes` stripped the whole quoted string, but perl interpolates
  `@{[…]}`/`${\…}` and array/hash SUBSCRIPTS inside double quotes, executing code. Fixed with a
  perl-aware `strip_inert_string_text` that KEEPS interpolated expression content for the
  `SAFE_PERL_WORDS` allowlist walk while dropping inert literal text — so `"system is down"` stays
  safe but interpolated `system`/backticks deny. Found by extending `interpreter_commands_deny_shell_escapes`.
- **Denylist "flush them out" guards.** (1) BEHAVIORAL: extended the interpreter-escape corpus with
  perl interpolation vectors. (2) SOURCE-LINT RATCHET: `no_new_denylist_named_constants_in_handlers`
  scans `src/handlers/**` for `*_DANGEROUS_*`/`*_MUTATING_*`/… constants; grandfather set only shrinks,
  a new one fails. Only `FIND_DANGEROUS_FLAGS` remains grandfathered.
- **git remote denylist → allowlist.** `check_git_remote` allowed everything except a MUTATING list —
  which omitted the `rm` alias of `remove` (fail-open!) and `set-head`/`update`. Converted to a
  read-only allowlist (`get-url`/`show`/bare/`-v`); mutating/unknown/aliased subs deny by omission.
- **tar denylist removed (superseded by the engine).** `tar` is engine-resolved (`resolve_tar`, a
  positive operation/locus model) and the engine is authoritative, so the legacy `is_safe_tar` flag
  DENYLIST (`TAR_DANGEROUS_*`) was dead code — its verdict always discarded. Gutted `tar.rs` to
  docs+registry+engine-integration-tests only.
- **mlr verb/flag data → TOML (`verb-chain` primitive):** replaced the `mlr.rs` handler (~66 verbs +
  main-flag `WordSet`s in Rust) with a declarative `[command.verb_chain]` grammar in
  `commands/data/mlr.toml` — strict `main_standalone`/`main_valued`/`main_variadic` flag regions + a
  `then`-chain over a `verbs` allowlist. New `DispatchKind::VerbChain` + `dispatch_verb_chain`;
  handler dir `handlers/coreutils/data/` deleted. All ~26 handler tests moved to TOML
  examples_safe/denied. GLOBAL guard `verb_chain_grammar_is_enforced_across_the_registry` enumerates
  every verb-chain command (auto-covers future ones): each allowlisted verb allows (bare + after
  `then`), a bogus verb denies, and an unknown MAIN flag denies at every position (generalizes the
  `-I` in-place hole). Red-demo proven. Documented in SAMPLE.toml.
- **cargo family `--manifest-path` gating (was: run a foreign project's build.rs/tests):** added a
  `Role::Exec` to the path-gate (gates a flag value by EXECUTOR locus via `execute_file_verdict` —
  denies `/tmp`/home/system where `write` allows `/tmp`) and declared `--manifest-path = "exec"` in
  cargo's command-level `[command.path_gate]`, so a foreign manifest denies UNIFORMLY across every
  cargo sub (build/test/bench/check/run/doc). `cargo run`'s per-sub `executor_redirect_flag` was
  dropped in favor of this one mechanism. Worktree/nested-crate manifests still allow.
- **cargo `--config` command injection (was: `target.*.runner`/`build.rustc-wrapper` code-exec):**
  `--config` removed from every cargo sub's valued list — `cargo X --config …` now denies. No safe
  subset is allowlistable (freeform keys + config-file paths), so it's dropped wholesale; legit config
  lives in `.cargo/config.toml`. Description documents the risk. Guard:
  `cargo_family_manifest_path_and_config_are_gated` (foreign manifest + `--config` deny across the
  family; worktree manifest allows).

- **Execution-origin (run the workspace's OWN code; deny foreign/inline/remote):** new developer-level
  `Execute` clause keyed on the EXECUTOR LOCUS (a two-sided-range locus band `[sandbox-scope,
  worktree-trusted]` — excludes `/tmp` below and home/system above for free); `execute_file_verdict`/
  `execute_project_verdict` engine entry points. Wired: `bash`/`sh` FILE, interpreters
  (`python3`/`node`/`ruby` via a declarative `executor = "file"` fallback + shared handler), `go run`
  (`file` sub with a `go-package` shape so a remote import path `pkg@version` denies — a real
  network-RCE hole found in adversarial review), `cargo run` (`project` sub + `--manifest-path`
  redirect gating). Interpreter inline (`-c`/`-e`/`-m`) stays denied. Guards: foreign-denies,
  worktree-allows, monotonicity, opaque-inline, unpinnable, glob-executor, go-run local-only,
  cargo family consistency, manifest-path redirect gating. Design:
  `docs/design/behavioral-taxonomy-execution-origin.md`.

- **mlr DSL verb safety (was: RCE via `put`/`filter` `system()`):** verb allowlist with `then`-chain
  parsing; `put`/`filter`/`step` (DSL) + `split`/`tee` (write) + unknown verbs denied by omission.
- **mlr variadic `--mfrom`:** modeled the `… --` terminator (read-only input files). `--mload` stays
  denied (loads DSL scripts = code).
- **sed `e` command:** `[addr]e command` (`1e id`, `p;e cmd`) now detected, not just trailing `e`.
- **Cross-command guard:** `handler_property_tests::interpreter_commands_deny_shell_escapes` — a
  corpus proptest flushing the whole "argument-is-code" KIND (mlr/awk/sed/perl/ruby/python/node/
  gnuplot). Demonstrated red (sed `1e id` allowed) → green.
- **sed `w`/`W`/`r`/`R` file commands (was HIGH/RCE):** full sed-script sub-parser (`scan_sed` in
  sed.rs) — tracks addresses, `s///`/`y///` bodies, blocks, `a`/`i`/`c` text — extracts `w`/`r`
  filenames and path-gates each by locus (local = SafeWrite/read, `/etc/cron.d/x` = deny). Wired into
  the authoritative engine `resolve_sed`. Also caught `[addr]e cmd` and the former `1e reboot`
  residual gap; unknown sed commands now fail closed. Guards: `write_mode_flags_deny_out_of_workspace_
  targets` (+ sed script writes) and `read_commands_deny_out_of_workspace_targets`. awk verified
  already solid (`print > file`, `print | "sh"`, `getline < file` all deny).
- **sed `-f` script files:** now DENIED (all forms) — align with `awk -f`/`bash`/`python`/mlr
  `--load`; the script content is unreadable so its `e`/`w`/`r` commands can't be verified.
- **mlr `step` verb:** now ALLOWED — verified pure (fixed named steppers via `-a`/`-f`/`-d`, no DSL).
- **magick bare-command PANIC (fuzz-found):** `magick` with no args hit `tokens[1]` out-of-bounds →
  classifier crash. Fixed with a `tokens.len() < 2` guard; swept all handlers, magick was the only
  one missing it. Caught by `handlers_never_panic_and_are_deterministic`.
- **sed glued/equals/cluster `-e` forms:** `scan_sed` now parses ALL script-supplying forms
  (`-eS`, `-neS`, `--expression=S`). Fixed a regression the `w`/`r` work introduced — the previous
  exact-match `-e` check let a glued form fall through, so the input FILE was scanned as a script and
  tripped the new unknown-command deny (`sed -eS file` wrongly denied). Both directions tested:
  legit glued/equals allow, hidden `w`/`e` in those forms deny.


## Output-flag write sweep — DONE + residual follow-ups DONE
The ungated-output-flag WRITE class is CLOSED: the 202-command sweep gated ~156 flag writers
(pathgates.toml [roles.X]); ~65 verified format-only rows remain on the ratchet worklist.
Residual follow-ups (this pass) also done -- 16 more gates:
  - handler/dir output flags: gs -o, mkdocs -d/--site-dir, cargo --target-dir/--out-dir,
    webpack -o/--output-path, vite --outDir, esbuild --outdir/--outfile, swc --out-dir, tsup.
  - positional last-arg writers (shape="last_write"): pdfunite, ps2pdf, pdf2ps, pdftops,
    sphinx-build, weasyprint, tiffcp, pdfcrop, lame, cjxl, djxl.
The ambiguous_output_flags guard's OUTPUT_FLAGS now also covers the unambiguous dir flags
(--outdir/--out-dir/--outDir/--target-dir/--site-dir/--output-dir/--output-path/--destination/--dest),
so those are enforced systematically; positional_and_output_dir_writers_gate_sensitive_paths regression-
guards the shape gates (not covered by every_declared_path_flag_actually_gates).
KNOWN SMALL TAIL (low severity, not blocking): obscure positional converters the flag guard can't
enumerate (a dedicated last_write audit would catch more); the single-char -d/-O set (kept out of the
guard for noise; specific ones gated); sub-positional writers like `hugo new site <path>`.

## Positional last-arg writer audit — DONE, with residual sub-classes
`positional_last_arg_writers_are_gated_or_acknowledged` (src/registry/tests.rs) drove a full-registry
audit of the last-positional / in-place WRITER class (the one the flag guard can't enumerate). Gated
~40 genuine writers via pathgates.toml [roles.X]:
  - converter families (shape="last_write"): the ghostscript wrappers (dvipdf, eps2eps, pdf2dsc,
    pfbtopfa, ps2epsi, ps2pdf12/13/14, ps2pdfwr, ps2ps), libtiff (pal2rgb, ppm2tiff, rgb2ycbcr,
    thumbnail, tiff2bw, tiff2icns, tiff2rgba, tiffcrop, tiffdither, tiffmedian), Little CMS
    jpgicc/tificc (shape merged onto their -o=read profile gate), and heif-thumbnailer, wkhtmltopdf,
    usdrecord, gdbm_dump, gdbm_load, pkgbuild, productbuild.
  - in-place mutators (shape="last_write", or positional="write" for multi-file): llvm-objcopy,
    llvm-strip, wasm-strip, install_name_tool, indent, resolveLinks, PlistBuddy, initdb,
    gatherheaderdoc; nbstripout + afscexpand (multi-file → positional="write").
The ~95 remaining auto-approvers are acknowledged NON-writers on tests/fixtures/positional_writer_worklist.tsv
(compilers/linkers → -o/a.out output, readers/viewers → stdout, test runners, linters, clipboard, flag
or derived output). The discovery ratchet is a description-heuristic best-effort (fail-OPEN on wording);
the fail-CLOSED guarantee for the known writers is positional_and_output_dir_writers_gate_sensitive_paths.

RESIDUAL SUB-CLASSES — RESOLVED (2026-07). On analysis none needed a brand-new primitive; each fit an
existing one, and the value-add was the proptests + one operation-aware mechanism.
  - ar-family (`ar`/`emar`/`llvm-ar`): NOT a first_write shape — a new pathgate `handler = "ar_archive"`
    (pathgate::handlers) reads the key-letter so r/q/d/m/s WRITE the archive and t/p/x READ it, and the
    add-ops read their members. Operation-awareness matters because read and write both deny a sensitive
    locus but DIVERGE at an in-workspace protected path (`.git/config`: readable, write-denied) — so a
    plain `positional = "write"` would over-deny `ar t ./.git/x.a`.
  - derived-output (`textutil`/`cap_mkdb`/`znew`/`pl2pm`): a sibling write lands in the input's directory,
    so write-gating the input path is locus-equivalent to gating the sibling. cap_mkdb/znew/pl2pm →
    `positional = "write"`. textutil has read modes too (`-info`/`-cat`) so it uses `handler =
    "textutil_mode"` (convert/strip write, info/cat read; -output/-outputdir are write targets).
  - scaffolders (`create-*`/`degit`): FACET model — a scaffolder CREATEs INERT CODE (the template is
    inert until the user runs it) into a NAMED directory. That is local SafeWrite; the axis to control is
    the LOCUS, so gate the target dir (`positional = "write"`, or last_write for degit) to keep the write
    in the workspace. Kept SafeWrite (not candidate) per the "inert code until run" nature; the npm-install
    step runs in the now-workspace-gated dir. (If we later want the install-exec itself gated, that is an
    execution-facet decision separate from this locus gate.)
  The operation-aware `handler` mechanism is guarded by pathgate_handler_names_resolve (name ⟺ fn) and
  proptests: ar/textutil "write is never more permissive than read" + "ops classify regardless of
  modifiers", with operation_aware_read_write_divergence_is_real pinning the .git/config case.

## `dispatch_executor` skips the flag policy when a positional is present

`ExecutorKind::File` returns `execute_file_verdict(first)` INSTEAD of `check_owned(tokens, policy)`,
so for any command declaring `executor = "file"` the flag policy — `max_positional`, the
standalone/valued allowlists — goes unenforced as soon as the first positional resolves. Only
positional COUNT and later positionals are affected; an unknown flag still denies, because it stops
`first_positional` resolving. (`ExecutorKind::Project` always checks the policy; `File` not doing so
looks like an oversight rather than a distinction.)

It cannot simply start calling `check_owned`: for an interpreter every token after the script is the
SCRIPT's argv (`python3 ./task.py --flag arg`), which the command's own grammar cannot describe.
That was tried and it false-denied. The fix is either to apply the policy to the tokens up to and
including the executor positional, or to declare which commands pass trailing args through.

Impact today is limited to commands that OPEN their extra positionals. `tilt` did — `tilt ./ok.erb
/etc/evil.erb` was admitted — and now carries `[command.path_gate] positional = "exec"` instead,
which composes with the grammar rather than replacing it. The interpreters (python3/ruby/node/go)
are unaffected: their trailing tokens are argv for workspace-local code, which the execution-origin
model trusts by design. `karma` had the same gap — `karma start ./ok.conf.js /etc/evil.conf.js` was admitted — and now
carries the same `path_gate`.

The structural half is now guarded: `capped_file_executors_declare_a_path_gate` (registry/tests.rs)
fails if any command declares a File executor WITH `max_positional` but no `path_gate`, so the next
one cannot inherit the hole silently.

DONE when: `dispatch_executor` enforces the policy over the pre-script prefix, and
`tilt a.erb b.erb` still denies with tilt's `path_gate` removed. Until then the guard is the
backstop, not the fix.

## Two targets cannot self-filter on the tool — verify their envelopes

`no_target_decides_on_a_foreign_tool` requires a target to abstain when the envelope names a tool
other than its shell tool. Seven targets do. Two are exempt because their envelope, as we model it,
carries NO tool identifier:

- **cursor** — flat `{command, cwd, workspace_roots}`; nothing names the tool.
- **grok** — `{toolInput:{command}, workspaceRoot, cwd, sessionId}`; HARNESS-BEHAVIORS.md's live
  verification records no tool field.

Neither was given an invented field name. HARNESS-BEHAVIORS.md's rule is that the harness wins and
these contracts were verified live, so guessing a key is the mistake that fails silently. Both rely
on their configured matcher, which is what every target did until recently.

Risk while exempt: both are deny-harnesses, so a foreign-tool envelope can only produce an
over-deny, never a grant.

(Antigravity was on this list and should not have been. Its identifier is `toolCall.name` =
`run_command`, documented AND live-verified in HARNESS-BEHAVIORS.md — we simply were not
deserializing it. Now filtered. The lesson for the two below: check the doc before assuming the
field is absent.)

DONE when: each harness's PreToolUse envelope is checked for a tool-identifying field (drive the
TUI, dump a real envelope for a non-shell tool). Either add the field and the filter — the guard
picks it up as soon as `sample_envelope` returns `Some` — or record in HARNESS-BEHAVIORS.md that the
envelope genuinely has none.

## `--setup` silently rewrites a wrong-typed key on codex / cursor / antigravity

Four targets (claude, qwen, droid, gemini) used to PANIC when an existing settings file had e.g.
`"hooks": "a string"`; they now go through `targets::append_hook_entry`, which reports the problem
and leaves the file untouched. Codex, cursor and antigravity never panicked because they guard with
`if !hooks.is_object() { *hooks = json!({}); }` — they REPLACE the user's value instead.

Replacing is milder than crashing but is still a silent destructive edit to a file we did not
write. An unreadable value usually means a hand-edit or a schema we do not know, and the same
argument that made the other four refuse applies here.

Not folded into the panic fix on purpose: those three have shipped, tested behaviour and two of
them nest differently (antigravity puts `PreToolUse` at the top level, with no outer key), so
`append_hook_entry` does not drop straight in.

DONE when: all seven refuse rather than overwrite — either by generalizing `append_hook_entry` to
an optional outer key, or by each guarding in place — and a test asserts the pre-existing value
survives, the way `refuses_a_wrong_typed_outer_key_without_panicking` does for the shared helper.

## `--suggest` appends to a `.safe-chains.toml` it cannot parse

`emit_suggestion` reads the existing file with `unwrap_or_default()`, merges the generated block in,
writes it back, reports "Added this to …", and prints a `[[trusted]]` pin. It never checks that the
existing content parses. Given a malformed file it produces a still-malformed one and tells the user
to pin it.

That used to chain into a fail-open: a pinned invalid file made `load_toml` panic on every
invocation, hook included, which lets the harness proceed. The panic half is fixed — such a file is
now skipped with a message — so the remaining damage is that `--suggest` claims success while
producing a file that will never load, and hands over a pin for it.

Same shape as the `--setup` panic already fixed: don't write into a config whose existing content we
could not read.

DONE when: `--suggest` validates the existing file before merging, and on failure reports the parse
error and writes nothing (the block can still be printed for the user to place by hand). A test
should assert the malformed file is byte-unchanged, as
`refuses_a_wrong_typed_outer_key_without_panicking` does for the install path.

## Re-tokenize split words instead of refusing them

An unquoted expansion whose value holds whitespace becomes several arguments at run time. Two
dimensions were leaking and are now handled differently:

- **Paths** — `locus::classify_local` classifies each split piece and takes the worst, so
  `VAR="x /etc/shadow"; cat $VAR` denies while `VAR="-rf ./sub"; rm $VAR` keeps its real locus.
- **Flags** — `check::smuggles_a_flag` REFUSES outright, because the danger (`fd --exec rm`,
  `find -exec rm {} \;`) is a capability the grammar would have rejected, not a place a path points.

Refusing costs a false deny on a value that hides a harmless flag: `VAR="-rf ./sub"; rm $VAR` is an
ordinary worktree delete and now prompts. Pinned as an accepted trade in
`an_unquoted_expansion_is_split_into_words`.

The precise fix is to re-tokenize: `Word::expand` already turns one word into many for brace
expansion, and feeding split pieces through it would let each command's own flag grammar judge them
— no refusal, no over-deny. The blocker is that a bound value carries SEPARATE read and write
representatives for loop variables (`loop_reprs`), so tokenization would have to choose a face
before the face is known.

DONE when: `Word::expand` splits unquoted variable expansions, `VAR="-rf ./sub"; rm $VAR` is allowed
again while `VAR="--exec rm"; fd pat $VAR` still denies, and `smuggles_a_flag` is deleted rather
than left as a second gate.
