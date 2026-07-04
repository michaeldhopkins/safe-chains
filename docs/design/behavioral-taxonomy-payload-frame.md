# The payload frame — decomposing "loads a config" / "calls an API"

Status: investigation (2026-07-04). Deepens pilot-2 R9 (declarative-config frame)
and R11 (API frame). Conclusion up front: these are **not two new frames and not
opaque**. They are one frame — *delegation to a typed payload in a remote-effect
language* — already named by v1.1's **interpreter frame** (SQL, `python -c`). The
work here is to stop saying "opaque by default" and describe the structure that
*is* knowable, and therefore allowlistable. Not yet implemented.

---

## 1. "Loads a config" is a pipeline, not an atom

`kubectl apply -f manifest.yaml` is four separable stages:

1. **Executor + verb** — `kubectl` / `apply`. The tool is trusted and known; the
   verb bounds the operation class (`get`=observe, `apply`=create/mutate,
   `delete`/`--prune`=destroy, `replace`=destroy+recreate).
2. **Payload source** — *where* the document comes from: `-f file`, `-f -`
   (stdin), `-f https://…` (URL), `-k dir` (kustomize tree), inline heredoc.
3. **Resource selector** — *what* is addressed: the `kind` + `namespace` in the
   doc; for an API, the URL path; for SQL, the table.
4. **Payload body** — the full document: the desired-state manifest / request body
   / SQL text.

"Loads a config" collapses all four into stage 4 and then gives up. But **stages
1–3 are on the command line or a shallow parse away**, and only stage 4 is deep —
and even stage 4 is a *grammar*, not a blob.

The same decomposition holds across the family:

| stage | `kubectl apply -f x.yaml` | `gh api -X DELETE /repos/o/r` | `psql -c 'DROP TABLE t'` |
|---|---|---|---|
| executor+verb | kubectl / `apply` | gh / `DELETE` | psql / `DROP` |
| source | `-f x.yaml` (file) | flags + path | `-c` (inline literal) |
| selector | `kind`,`namespace` | `/repos/o/r` | table `t` |
| body | manifest | request body | SQL text |

They are the same frame. `apply -f` is the **batch/convergent** variant (a *set* of
resources, plus implicit deletes via `--prune`); `gh api`/`psql -c` are the
**atomic** variant (one verb, one resource). Batch is looser (unbounded scale,
implicit destroy); atomic is a single statement. One family, two intensities.

---

## 2. Where the verb lives is the real variance

The single most useful distinction is **whether the verb+selector are exposed
structurally or buried in the body**:

- **Structured surface** — `aws ec2 run-instances`, `kubectl delete ns/prod`,
  `gcloud compute instances create`. The verb and resource are *subcommands and
  arguments*. This is **already** in our subcommand-tree model; it is allowlisted
  today. `aws s3 ls` is trivially safe, `aws s3 rb` is not, and we can tell from
  the tree.
- **Raw escape hatch** — `gh api -X …`, `kubectl --raw`/`apply -f`, `psql -c`,
  `aws … --cli-input-json`, `curl` against the same REST API. The verb+selector
  move *into the payload*. This is the only genuinely hard case.

So the frame's first job is to **prefer the structured surface**: a tool that
exposes verb+resource as subcommands needs no payload parsing. The raw hatch is
where stages 2–4 must be classified or worst-cased. This is a concrete,
non-arbitrary rule, not a vibe: *classify by the structured surface when the tool
offers one; engage the payload machinery only for the raw hatch.*

---

## 3. What is allowlistable — four gates

A payload-frame capability can be constrained at each stage. The gates compose
(all must pass); each is a small allowlist, in the project's existing idiom.

**Gate 1 — verb.** `method ∈ {GET, POST}` / `subcommand ∈ {get, apply}` /
`sql_verb ∈ {SELECT}`. On the command line for structured tools; a shallow parse
for `-X`/`-c`. Bounds the operation regardless of body.

**Gate 2 — source integrity.** This is the execution-provenance axis (§2.6)
applied to the *input document*:

| source | integrity | note |
|---|---|---|
| inline heredoc / `-c 'lit'` | `caller-inline` | the human typed it |
| in-repo pinned file | `worktree` / `worktree-trusted` if under a trusted-config root | TOCTOU-stable if pinned |
| `-f https://…` | `network-sourced` | untrusted; also mutable after check |
| `-f -` (stdin) | `unknown` | worst-case |

Gate 2 is what decides whether parsing the body is even worth it: you can parse a
URL-fetched manifest, but you cannot *trust* it won't change between check and
apply. Low source integrity → worst-case, skip gates 3–4.

**Gate 3 — resource selector.** `path matches /repos/*/issues` /
`kind ∈ {Deployment, Service, ConfigMap}` / `namespace ∈ {dev, staging}` /
`table ∉ {users, secrets}`. A pattern allowlist over the selector — the same
shape as a filesystem-locus allowlist, one domain over.

**Gate 4 — payload predicates.** A **nested-grammar allowlist** over the body:
forbid `privileged`/`hostPath`/`hostNetwork` in a Pod spec; forbid `verbs: ["*"]`
in an RBAC rule; forbid `DROP`/`GRANT` in SQL. This is safe-chains' own
architecture recursed one level: a grammar + an allowlist + a worst-case fallback,
applied to k8s/HCL/SQL instead of shell. Expensive per language (HP-7), but the
frame gives each payload language a *home and a uniform contract*, so they can be
added incrementally, highest-value first.

Sketch, in the level-TOML idiom (§4.1), for a hypothetical safe clause:

```toml
[[level.developer.allow]]          # "kubectl apply of benign dev workloads"
delegate    = "payload"
verb        = ["apply", "get", "diff"]        # gate 1
source      = "<= worktree"                    # gate 2: in-repo files only, no URL/stdin
selector    = { kind = ["Deployment","Service","ConfigMap","Secret?no"],
                namespace = ["dev","staging"] }# gate 3
payload_deny = ["privileged","hostPath","hostNetwork","ClusterRoleBinding"]  # gate 4
```

Everything not matched by such a clause stays denied — the allowlist floor is
intact; we have only described *more* of the safe region precisely.

### 3.5 The gates are a resolution ladder — most levels stop early

The four gates are not a checklist every command runs; they are **increasing
resolution**, and a level engages only as deep as it needs (v1.1 §4.4). This is
what keeps the frame from forcing a universal interpreter:

| depth | knows | cost | who stops here |
|---|---|---|---|
| **R0** presence | *there is a payload* | free | `payload-forbid` levels: deny and done |
| **R1** verb + source | type, integrity, operation-bound | shallow parse of the command line | `payload-blind` levels: "trusted-source payload is fine" — allow and done |
| **R2** selector | which resource/path/table | selector parse (often on the command line) | levels that gate by namespace/path/verb |
| **R3** body | content predicates | full payload-grammar parse | only `payload-aware` levels, only for languages with a resolver |

Only **R3** needs a language grammar, and only the levels that ask for content
predicates ever trigger it. A language with no R3 resolver simply can't satisfy an
R3 clause → that clause denies → the allowlist floor holds. So payload grammars
(k8s, HCL, SQL, …) are an **optional, incremental library** — add the highest-value
one first, ship the rest never — not a prerequisite. The frame's job is to *carry
the resolution level*, so a capability can honestly say "I accept a k8s payload
from an in-repo file" (R1) without anyone having to parse the manifest.

---

## 4. How this fits input → capability and state → trust

This is the frame's real place in the model.

### 4.1 Input → capability: the payload is a nested program
There is a spectrum of how much input determines behavior:

```
fixed  →  facet-parameterizing  →  behavior-carrying
git status   rm $PATH (input=locus)    kubectl apply -f X / psql -c / bash -c
```

Payload delegation is the **behavior-carrying** end: the input *is* a program in a
remote-effect language. The model's answer is **recursion** — classify the nested
grammar with the same machinery (gates 3–4 are an allowlist over a grammar). A
safe-chains that classifies a k8s manifest is a safe-chains that classifies a
shell line, one level down. This is why the frame is not a special case; it is the
core model applied reflexively.

### 4.2 State → trust: two different states, two different roles
The user's question — *how does state inform capability and trust here* — resolves
into two distinct states that the word "state" hides:

- **Source state = trust in the input.** The provenance of the *document* (gate 2)
  decides whether the body-allowlist verdict can be believed. Inline > pinned
  in-repo > URL > stdin. This is integrity/provenance, and it ties directly to the
  v0.205.0 trusted-config model: a manifest under a hash-pinned trusted root is
  `worktree-trusted`; the same bytes fetched from a URL are not.

- **Target state = the locus, and it is usually *ambient*.** Which cluster,
  account, or database the command hits is set by `kubectl` context / `AWS_PROFILE`
  / `$KUBECONFIG` / `gcloud config` / `$DATABASE_URL` — **session/environment
  state, not the command line.** `kubectl apply -f x.yaml` is harmless against a
  kind cluster and catastrophic against prod, and *the discriminator is invisible
  to the checker*. The blast radius is a property of state the command never names.

That second point is a genuine gap, logged as **HP-12**. It is the sharpest answer
to "how does state inform capability": for every remote/payload command, the most
important facet (locus = which remote) lives in ambient state we don't read, and
reading it (kubeconfig, env) is itself fraught (it can change; reading may have
cost). It generalizes HP-4 (env reinterprets commands) specifically to *blast
radius*.

---

## 5. Findings

- **Reframe R9 + R11 → one enriched frame.** Do not add "declarative-config" and
  "API" as separate frames. Enrich v1.1's **interpreter frame** into the
  **payload frame** with the four-gate decomposition (§3). Fewer frames, more
  precision — and it stops the "opaque by default" undersell.
- **The structured-surface rule (§2).** Classify via subcommands where the tool
  exposes verb+resource structurally (already our model); engage payload parsing
  only for the raw hatch (`-X`, `-c`, `--raw`, `-f -`).
- **Gate 2 links to the trust-config model.** Source integrity is the existing
  provenance axis applied to the input document; a trusted-config root promotes an
  in-repo payload to `worktree-trusted`.
- **Gate 4 is safe-chains recursed.** Payload-language allowlists (k8s/HCL/SQL) are
  the same grammar+allowlist+fallback architecture, one level down; addable
  incrementally under a uniform contract.
- **New HP-12** — *ambient-state target locus*: the remote a payload/API command
  hits is set by session state (kube-context, `AWS_PROFILE`, `$DATABASE_URL`), not
  the command line, so the dominant facet (which remote / prod-vs-dev) is invisible
  to the checker.
