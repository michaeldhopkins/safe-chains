# TODO

## THE campaign — re-research every command (see RESEARCH-PLAN.md)

Decision (2026-07-16): re-research and upgrade the TOML of EVERY command under the facet model. No
shortcuts — the level-based tail hid real credential exposures (`vault read`, `security
find-internet-password`, `aws secretsmanager get-secret-value`). Batched, highest-risk-first, with a
targeted adversarial review after each batch and a general facet-vocab assessment every ~2–3 batches.
Full plan, standard, batch order, and cadence in **RESEARCH-PLAN.md**. Next: **Batch 0 (credential
slice)** — classify the 17 subs on the `credential_smelling_subs_*` guard's grandfather worklist.

## Pre-1.0 hardening

- **FAN-OUT PRIORITY — credential-exposure audit (found in the pre-fanout adversarial review).** Many
  cloud-CLI subs READ or MINT credentials but auto-approve at the default band, because they predate the
  `credential-read`/`credential-mint` archetypes. Two vectors: (a) explicit subs classified SafeRead
  (fixed exemplars: `gcloud auth print-access-token`/`print-identity-token`, `vault read`); (b) BROAD
  glob read-patterns — `first_arg = ["get-*"]` admits `aws secretsmanager get-secret-value` and `aws ecr
  get-login-password` (both confirmed auto-approving). The fan-out MUST, per cloud CLI: tag credential
  read/mint subs `credential-read`/`credential-mint`, and audit every `get-*`/`describe-*` glob for
  credential actions (get-secret-value, get-login-password, get-session-token, get-token, get-password).
  This is the #1 correctness item before/within the fan-out.
- **Over-deny audit follow-ups.** A real one was found+fixed (grep `-P`/`--perl-regexp`, DONE below).
  Remaining, from the real-world sweep — coverage GAPS (not false-denies of covered ops):
  - `terraform` isn't covered at all — add read-only subs (`plan`/`validate`/`show`/`fmt`/`output`
    /`state list`), deny `apply`/`destroy`.
  - `fd -x <safecmd>` doesn't delegate its exec flag the way `find -exec` does — add delegation.
  - Judgment calls (running opaque code — same "vettable vs opaque" line as `./bill`, likely keep
    denying): `npm run <script>`, `pnpm install` (postinstall), `python3 -m <module>`.
- **`.safe-chains.toml` protected third-party config location** (open research item, noted in
  `regions/default.toml`): the trust root lives at `~/.config/safe-chains.toml`, an UNPROTECTED path
  (a python/editor write escapes the command classifier). A harness-blessed protected location would
  close the residual. Until then it's best-effort — the command classifier denies every *command*
  write (now guarded), but not a non-command write.
- **(Optional) dedicated cargo-fuzz target.** The in-repo proptest + adversarial-corpus fuzzing now
  covers panics/hangs and found two real bugs; a libFuzzer target would go deeper still, but needs
  nightly + a `fuzz/` crate + CI wiring — a separate infra decision.

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

## Hook level: support LOWERING the ceiling (stricter than developer)
`configured_hook_level` today honors only UPPER levels (raise the ceiling). A configured
`reader`/`editor` is a request for a STRICTER auto-approve band, but the hook can't honor it
yet: the CLI's `command_verdict` + `<= threshold` gate handles the engine verdict correctly
(a redirect write projects to SafeWrite → gated at SafeRead), but the hook's extra
`explain_with_coverage` legacy-coverage fallback would re-admit a write the ceiling denied
(it flattens covered commands to `Inert`). To support lowering: mirror `run_cli`'s
`(threshold, upper_level)` + gate in the hook AND gate/skip the legacy fallback when the
configured ceiling is below developer. Then `level = "editor"` would gate a sibling write
through the hook (the engine already distinguishes editor≠developer; only the projection
can't yet) — closing the editor/developer collapse for lower plans too.
