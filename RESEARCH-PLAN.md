# The every-command re-research campaign

**Goal:** re-research and upgrade the TOML of *every* command in safe-chains under the facet/behavior
model. No shortcuts. The level-based tail was classified over a long period, much of it before the
facet model surfaced the distinctions we now rely on (remote reads, exfil, credential exposure,
execution-origin, machine sub-rungs, install safety). `vault read`, `security find-internet-password`,
and the `aws secretsmanager get-*` glob were all found auto-approving credential exposures during
setup — proof the tail can't be trusted on faith.

This is not "convert everything to archetypes." A genuinely simple tool (`jq`, `bat`, a formatter) is
correctly `level = "SafeRead/SafeWrite/Inert"` — that IS its facet classification. The job is to
*verify every command under the facet lens* and upgrade the ones that hid a facet.

## Per-command research standard (the checklist)

For each command, done means:

1. **Research the LATEST upstream version** (GitHub releases / npm / docs), not the local install.
   Record it in `researched_version` — this is the completion marker and the next person's tripwire.
2. **Characterize the WHOLE surface** — every subcommand and every behavior-changing flag — along the
   facet axes (operation · locus incl. remote & the machine sub-rungs · authority · reversibility ·
   persistence · disclosure · secret · network · execution · cost). Do NOT reason in "read-only vs the
   rest".
3. **Map each sub to its home:**
   - operand/flag-independent, local, simple → keep `level = …` (the facet shorthand).
   - above-the-line or operand-dependent → `profile = "<archetype>"` + cited `fact` / `source`
     (`judgment` where we make a policy call). Archetypes:
     `remote-{read,create,mutate,destroy-recoverable,destroy-irreversible,authorize,control,exec}`,
     `credential-{read,mint}`, `vcs-sync`, `{local-install-pinned, supply-chain-build}`,
     `blockchain-txn`, `{local-privileged, privileged-control}`.
4. **Run the mandatory sub-audits** (each is a known failure mode):
   - **Credential audit** — any sub that READS/PRINTS or MINTS credential material →
     `credential-read`/`credential-mint`. Includes broad globs: a `first_arg = ["get-*"]` that admits
     `get-secret-value` / `get-login-password` / `get-token` / `get-password` must be narrowed.
   - **Network reach** — a cloud read tagged local `SafeRead` is really `remote-read`.
   - **Destination** — a send target from a flag (`--repo`) or positional → `network_destination`.
   - **Install** — pinned + scripts-off → `local-install-pinned`, else `supply-chain-build`.
   - **Execution** — runs foreign/remote code → the execution facet / `remote-exec`.
5. **Write `examples_safe` / `examples_denied`** that pin the intended verdicts.
6. **Green:** `cargo test`, `cargo clippy -- -D warnings`, and the ratchet guards below.

## Batch order (highest blast-radius first)

Mis-classification cost is highest where it auto-approves a hole, so front-load those.

- **Batch 0 — Credential slice (cross-cutting).** The `credential_smelling_subs_*` guard's worklist
  (17 subs today) + any `secret`/`token`/`key`/`password` sub-group's read/mint sub-subs across all
  files. Sharpest risk (exposures auto-approve). Shrinks the guard's grandfather set to the confirmed-
  benign core.
- **Batch 1 — Cloud / infra CLIs.** aws, gcloud, az, kubectl, oci, terraform, tofu, pulumi, helm,
  docker, and the ~256 candidate-bearing files. Remote writes, credentials, remote-exec. (The "fan-out".)
- **Batch 2 — Package managers / toolchains.** npm (done), pip, cargo, gem, bundler, yarn, pnpm, go,
  nix, spack. Install safety (the `when_absent` scripts-off pattern per manager).
- **Batch 3 — VCS.** git (done), svn, hg, fossil. Destination/exfil.
- **Batch 4 — Containers / orchestration.** docker, nerdctl, podman, oc, istioctl, k8s tooling.
- **Batch 5 — System / privileged.** systemctl (done), launchctl, security, defaults, networksetup.
- **Batch 6 — Blockchain, ML, db, serverless, migrations, forges.**
- **Batch 7 — The simple-tool tail.** coreutils (done), jq, bat, formatters, linters — verify each is
  genuinely operand-independent; most stay level-based. Fast but not skipped.

## Cadence — review in batches, pause to assess

- **After every batch:** run the full guard suite + a *targeted adversarial review* of that batch's new
  classifications — actively try to evade them (the npm `--ignore-scripts=false` / git `--repo=ext::`
  probes are the template). Fix findings before moving on.
- **Every ~2–3 batches (or when the facet vocab feels strained):** a **general assessment** — is the
  archetype/level/facet vocabulary still sufficient? Refine it (add facets/archetypes, fix bugs) before
  continuing. We EXPECT the vocabulary to evolve; capture each change in the design docs + memory.

## Tracking & guards

- **Completion marker:** `researched_version` present on a `[[command]]` = re-researched under this
  campaign. (Coverage today is sparse — most commands lack it.)
- **Ratchet guards (grow/shrink only in the safe direction):**
  - `credential_smelling_subs_are_classified_or_grandfathered` — the credential class; grandfather set
    only shrinks. Would have caught `vault read`.
  - `no_new_denylist_named_constants_in_handlers` (existing) — handler denylist ratchet.
  - `every_*` registry guards (path-flag roles, eval-safe, etc.) — structural invariants.
  - *(to build as needed)* a network-mismatch triage (SafeRead + "network/remote/API" in the
    description → review as remote-read), and a coverage report (commands missing `researched_version`).

## Status

- Setup done: credential-smell ratchet built; the fan-out vocabulary is complete (credential/
  remote-exec archetypes, install clause + `when_absent`, machine sub-rungs, per-level threshold).
- **Batch 0 (credential slice) — largely done.** Classified `credential-read`: `vault read`, macOS
  `security find-generic/internet-password`, `gcloud auth print-access/identity-token`, `aws configure
  export-credentials`. Closed value-reading globs (fail-closed; full restructure = Batch 1): `aws
  secretsmanager get-secret-value`, `gcloud secrets versions access`. Confirmed benign (permanent
  grandfather): caddy hash-password, platform/upsun auth:api-token-login, please recache-token,
  rails/rake `secret` (random generator), koyeb/wrangler `secret[s]` (write-only), clever `tokens`.
  The 3 research-TODOs are now DONE (verified against upstream docs, not guessed): `basecamp auth
  token` PRINTS the OAuth token → `credential-read`; `istioctl proxy-config secret` inspects the
  proxy's TLS material (`-o json` dumps private keys) → `credential-read`; `dcli team credentials` is
  team credential-SHARING audit metadata (no vault values) → confirmed benign. **Batch 0 complete.**
  Bonus finding for Batch 4: `istioctl proxy-config all -o json` also dumps secrets — a flag-conditional
  refinement (metadata default vs `-o json` key dump) is the accurate long-term model.
- Batch-0 debrief (feeds the cadence): (1) the guard caught more than grep, cross-file; (2) real
  judgment call surfaced — `aws export-credentials` was `Inert`+`eval_safe`, now `credential-read`,
  which BREAKS the assume-role→export→eval workflow at the default band (flagged to revisit); (3)
  cloud `get-*`/`versions` globs hide credential-value reads — every such glob needs a Batch-1
  sub-sub restructure; (4) process bug: a line-based edit missed a TOML `[…eval_safe_flag_values]`
  sub-table → registry panic that masked ALL commands until the full test suite caught it (verify
  after every edit; the panic cascades loudly, which is good).
- **Batch 1 TRIAL (multi-agent workflow) — 5 cloud/PaaS CLIs done.** railway, render, neon, porter,
  supabase re-researched by parallel agents (research→adversarial-verify pipeline), then verified +
  integrated by the orchestrator. ~146 candidate subs classified. New credential-read holes CLOSED:
  railway `variable list`/`variables` (env vars = secret store — caught by the VERIFY agent, missed by
  research), railway `bucket credentials` (S3 keys), neon `connection-string` (prints DB password),
  porter `config --show-token` (raw API token — the research agent found+closed it itself), supabase
  `projects api-keys`/`encryption get-root-key`/`gen keys` (credential-read) + `gen signing-key`
  (credential-mint). supabase `secrets list` correctly NARROWED to remote-read (names+digest only).
  All green (build + tests + guards + clippy).
  - **DECISION RESOLVED — cloud-read convention = `remote-read`.** User: "We should be using
    remote-read if it is an accurate facet." So cloud reads standardize on `profile = "remote-read"`
    (facet-accurate); the level-based files (render/porter/koyeb/gcloud/aws) migrate for consistency as
    they're re-researched. A profiled sub is engine-classified and ignores its flag allowlist, so it
    stopped rejecting unknown flags → profiled subs are exempted from `toml_specs_reject_unknown` (same
    principle as the corpus-gate skip; flag-danger is per-flag-escalation's job + the adversarial review).
  - **DECISION RESOLVED — `db dump` = the new `data-export` archetype (BUILT).** A dump is more than a
    get-URL read: it's a BULK export (volume) that can also WRITE A FILE (`-f`). New archetype
    `data-export` (observe·remote·**scale=unbounded**·disclosure→local-process) records the volume;
    `scale × disclosure` proxies the data's unknowable sensitivity (recognize-and-record, never judge).
    New declarative field `output_path_flags` on the sub adds a SECOND, path-gated local write for the
    `-f`/`--file` output — `-f ./out.sql` stays worktree-local (auto-approves), `-f /etc/passwd` gates
    on locus (denied). Glued short `-fPATH` matched too (bypass-closed, red→green). supabase `db dump`
    migrated. Per the user's steer ("not saying to deny it"): recorded accurately, the bulk stdout read
    still auto-approves; a scale-gate on remote bulk egress is a deferred level-design call.
  - **OPEN MEDIUM:** railway `environment delete` under-tagged recoverable (cascades to irreversible
    volume data). LOWs: interactive `nested_bare` TUI-escape (render services), group over-denies
    (skills/workflows/datastore/endpoints read leaves), archetype-precision nits.
  - **Follow-up when the export family grows:** pg_dump/mysqldump/`<cloud> db export` reuse
    `data-export`; list EVERY output-path flag per tool (a missed one drops its write-gate) and add a
    corpus guard over data-export subs once there's more than one.
  - **Trial verdict:** the flow works — agents did genuine per-CLI research with citations and left
    the undeterminable as candidate (no guessing); the adversarial-verify stage caught the one real
    miss (railway variable). ~671k tokens / ~49 min for 5 files (supabase, 66 subs, dominated).
- **Batch 1 — AWS (in progress).** Different shape from the PaaS CLIs: 249 service subs, each
  auto-approving read-verb GLOBS (`get-*`/`describe-*`/`list-*`), no per-action sub-subs. The risk is a
  glob silently admitting a credential/secret/bulk action (the `secretsmanager get-secret-value` hole,
  generalized). Ran a **sharded triage→adversarial-verify sweep** over all 248 services (404 agents,
  ~13.3M tokens, 0 errors), each agent PROBING the real `safe-chains` binary as ground truth.
  - **Result: 93 unique confirmed holes across 54 services; vocabulary FLATTENED** — 92/93 map to
    existing categories (35 credential-read, 32 credential-mint, 14 sensitive-disclosure→credential-read,
    12 data-export-splits). Verify killed the real false positives (`chime get-attendee` moved to
    chime-sdk-meetings; `lightsail get-bucket-access-keys` returns only the key ID). Worklist saved:
    scratchpad `aws_holes_deduped.json`.
  - **Enabling fix (SHIPPED):** `build_sub_kind` used to DROP a sub's `first_arg` glob when it had
    sub-subs — so "glob + carve-out" was inexpressible at the sub level. Now it threads `first_arg`
    (mirrors the command level). A service can carry its `get-*` glob AND carve dangerous actions to
    profiled sub-subs; dispatch checks carve-outs first (dispatch.rs), so they escalate while benign
    reads still glob-match. No registry-wide regression.
  - **The one refinement AWS surfaced — `bulk-object-read` archetype (SHIPPED).** `s3api get-object`
    (= the not-allowlisted `s3 cp`) auto-approved via the glob. `data-export` auto-approves (wrong for
    arbitrary blobs). User decision: blobs DENY, structured query-results stay auto-approving, db dump
    unchanged. New archetype `bulk-object-read` (secret=reads → yolo) for s3api get-object / glacier
    get-job-output / ebs get-snapshot-block / omics get-read-set. **NUANCE FLAGGED BY USER (unsettled):**
    yolo is the same tier as credential-read and arguably too strict — "not convinced all blobs are
    yolo-level." No facet basis for a proportionate middle tier (a remote fetch-read is reader-level;
    only `secret` lifts it, overshooting to yolo). Recorded in the archetype `because`, the design doc
    §2, and each sub-sub's `judgment`. Revisit if a "content vs metadata" axis is added.
  - **ALL 93 holes handled (COMPLETE, verified end-to-end).** 86 carved out (72 straight credential-
    read/mint via a generated splice + 4 flag-conditional escalators + ecr ×2 + ssm ×4 + 4
    bulk-object-read blobs); 7 query-result readers intentionally left auto-approving (your-data reads,
    consistent with db dump). Verification: 93 holes → 101 outcomes (base+flagged), 0 unexpected; 51
    benign glob-siblings still auto-approve, 0 broken; 13 adversarial evasion probes (glued/negated/
    value-gated flag forms) all correct.
  - **Property guard (SHIPPED, non-vacuous red→green):** `glob_carveouts_deny_while_the_glob_still_allows_siblings`
    in registry/tests.rs walks the real registry — every credential-/blob-profile carve-out under a glob
    service must deny, every remote-read base must allow, every glob must still allow a synthetic benign
    sibling. Covers this batch AND future AWS carve-outs automatically. 4218 tests, clippy clean.
  - **Adversarial-review note (UX, not a hole):** `lambda list-functions`/`get-function`/
    `get-function-configuration` now DENY (they always return plaintext `Environment.Variables`). Correct
    per the sensitive-disclosure→credential-read policy but aggressive for routine ops — a possible future
    "env-vars vs metadata" refinement, same flavor as the bulk-object-read tier nuance.
  - **Residue sweep — done DETERMINISTICALLY, not another LLM run.** Extracted every read-verb AWS action
    whose NAME smells of credentials from the bundled botocore models, probed each. It flushed 2 real holes
    the LLM sweep MISSED — `ssm get-access-token` ("credentials set for just-in-time node access") and
    `lakeformation get-temporary-data-location-credentials` (temp AWS creds) — now carved. 4 other
    credential-smell hits were false positives (verified via botocore OUTPUT SHAPES: `chime-sdk-voice
    list-...-termination-credentials` = usernames only; `cognito-idp list-user-pool-client-secrets` = doc
    says never reveals the secret; `wafv2 get-decrypted-api-key` = token-domains only; `wafv2 list-api-keys`
    = public CAPTCHA integration tokens) → grandfathered.
  - **PERMANENT GUARD (SHIPPED, red→green):** `aws_credential_smell_actions_deny_or_are_grandfathered`
    (registry/tests.rs) + fixture `tests/fixtures/aws_credential_smell_actions.tsv` (60 rows). Every
    credential-smell AWS action must DENY or be GRANDFATHERED with a verified reason; a new one that
    auto-approves fails the test. **Regenerate the fixture** (when re-researching AWS) from botocore:
    ```
    DATA=/usr/local/aws-cli/awscli/botocore/data   # aws-cli bundled models
    for d in "$DATA"/*/; do svc=$(basename "$d"); ver=$(ls "$d"|sort|tail -1)
      jq -r --arg s "$svc" '.operations|keys[]|"\($s)\t\(.)"' "$d$ver/service-2.json"; done \
    | grep -iE 'Credential|SessionToken|FederationToken|AuthorizationToken|AccessToken|LoginPassword|Password|Secret|PrivateKey|SigningKey|StreamKey|InstanceAccess|ComputeAccess|OpenId|ConnectionString|MasterUser|ApiKey|APIKey|GetLogin' \
    | awk -F'\t' 'BEGIN{OFS="\t"}{k=$2; gsub(/([a-z0-9])([A-Z])/,"\\1-\\2",k); gsub(/([A-Z]+)([A-Z][a-z])/,"\\1-\\2",k); print $1, tolower(k)}' \
    | awk -F'\t' '$2 ~ /^(get|describe|list|batch-get|head|filter)-/' | sort -u
    ```
    then intersect with real CLI service names, triage new rows (carve or grandfather with a botocore
    output-shape check — `.operations[Op].output.shape` → `.shapes[Shape].members`).
- **AWS COMPLETE.** Next: gcloud/az/kubectl + the remaining candidate files (reuse the patterns; port the
  botocore-style credential-smell guard per cloud where a machine-readable API model exists).
