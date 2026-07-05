# The Kubernetes R3 payload-body resolver

Status: design (2026-07-05). Designs the one payload-body (gate-4) resolver the
survey recommended building first (`behavioral-taxonomy-payload-survey` §1): a
bounded Pod-Security-Standards + RBAC danger-scan invoked only when a level's clause
descends to R3 (`…-payload-frame` §3.5). Targets Kubernetes 1.34. Not yet
implemented.

## 1. Inputs & scope

The resolver is the R3 stage of the payload frame (`…-payload-frame` §3). It runs
**only after** gates 1–2 pass and gate 3 is being evaluated:

| gate | already decided when R3 runs |
|---|---|
| 1 verb | `kubectl apply` / `create` / `replace -f` — an admissible create-or-mutate verb. `delete`/`--prune`/`--raw` mutating writes are handled at R1 and never reach here as an admit path. |
| 2 source | the document set has acceptable integrity: `worktree`/`worktree-trusted` in-repo file, or `caller-inline` heredoc. `network-sourced` (`-f https://`) and `unknown` (`-f -` stdin) are worst-cased at R1 and do **not** enter R3. |
| 3 selector | `kind` + `metadata.namespace`; the namespace pattern gate is on the command line (`-n`) or the doc. R3 supplies `kind` back to gate 3. |

**Input.** A byte buffer that is a set of manifest documents plus a declared source
form:

| source form | R3 action |
|---|---|
| single/multi-doc YAML or JSON file (`-f x.yaml`), directory (`-f dir/`, recurses `*.yaml`/`*.yml`/`*.json`) | parse and scan every document |
| `-k dir/` (kustomize) | requires `kustomize build` render first — see §3 |
| helm chart | requires `helm template` render first — see §3 |
| `-f -` / `-f https://` | never reaches R3 (gate 2) |

**Output.** Per document set, a bounded record:

```
R3Result {
  clause: Admit | Deny,              // the R3 clause verdict, allowlist floor default = Deny
  kinds:  Set<Kind>,                 // every kind observed (incl. recursed pod templates)
  controls: Set<Control>,            // dangerous-control flags present (§2)
  facets: Set<Facet>,                // facet contributions: locus | authorize | secret | exec
}
```

`clause = Admit` iff every document parsed, every `kind` is known, and
`controls ∩ (clause's deny set) = ∅`. Any parse failure, unknown `kind`, or
unevaluable control forces `Deny` (§5). The resolver never emits `Admit` on
incomplete information.

Facet mapping is the resolver's contribution to the behavioral model: a `hostPath`
volume or `hostNetwork` Pod contributes **locus** (host/blast-radius escalation); a
`ClusterRoleBinding`/wildcard rule contributes **authorize**; `kind: Secret`
contributes **secret**; a `pods/exec`-granting rule or `data`-style exec sink
contributes **exec**.

## 2. Discriminator set (Kubernetes 1.34)

Two externally-specified allowlists supply the predicate set: **Pod Security
Standards** (Baseline ⊂ Restricted) and **RBAC good-practices**. Each row is a fixed
dotted-path scan.

### 2.1 Pod Security Standards — container/pod controls

Paths below elide the `spec.` prefix and the container-array fan-out: each applies at
`spec.containers[*]`, `spec.initContainers[*]`, and `spec.ephemeralContainers[*]`
unless it is a pod-level field. All resolve against the pod spec, wherever nested
(§3).

| control | field path(s) | violating value | profile | facet |
|---|---|---|---|---|
| Privileged | `…securityContext.privileged` | `true` | Baseline | locus, exec |
| HostProcess (Windows) | `…securityContext.windowsOptions.hostProcess` | `true` | Baseline | locus, exec |
| Host namespaces | `hostNetwork`, `hostPID`, `hostIPC` (pod-level) | `true` | Baseline | locus |
| HostPath volume | `volumes[*].hostPath` | present | Baseline | locus |
| Host ports | `…ports[*].hostPort` | non-nil, ≠ 0 | Baseline | locus |
| Added capabilities | `…securityContext.capabilities.add` | any value outside the Baseline set† | Baseline | locus |
| Probe/lifecycle host | `…{liveness,readiness,startup}Probe.{httpGet,tcpSocket}.host`, `…lifecycle.{postStart,preStop}.{httpGet,tcpSocket}.host` | non-empty (1.34 Baseline) | Baseline | locus |
| /proc mount | `…securityContext.procMount` | any but `Default` | Baseline | locus |
| Seccomp (baseline) | `…securityContext.seccompProfile.type` | `Unconfined` | Baseline | locus |
| AppArmor | `…securityContext.appArmorProfile.type` + annotation `container.apparmor.security.beta.kubernetes.io/*` | any but `RuntimeDefault`/`Localhost` (`runtime/default`/`localhost/*`) | Baseline | locus |
| SELinux type | `…securityContext.seLinuxOptions.type` | outside `{container_t, container_init_t, container_kvm_t, container_engine_t}` | Baseline | locus |
| SELinux user/role | `…securityContext.seLinuxOptions.{user,role}` | non-empty | Baseline | locus |
| Sysctls | `securityContext.sysctls[*].name` (pod-level) | outside the Baseline safe-sysctl set‡ | Baseline | locus |
| allowPrivilegeEscalation | `…securityContext.allowPrivilegeEscalation` | not `false` | Restricted | locus |
| runAsNonRoot | `…securityContext.runAsNonRoot` (pod or container) | not `true` | Restricted | locus |
| runAsUser = 0 | `…securityContext.runAsUser` | `0` | Restricted | locus |
| Seccomp required | `…securityContext.seccompProfile.type` | unset (must be `RuntimeDefault`/`Localhost`) | Restricted | locus |
| Capabilities drop-ALL | `…securityContext.capabilities.drop` | does not include `ALL` | Restricted | locus |
| Volume types | `volumes[*]` | any volume type outside the Restricted set§ | Restricted | locus |

† Baseline-allowed `add`: `AUDIT_WRITE, CHOWN, DAC_OVERRIDE, FOWNER, FSETID, KILL,
MKNOD, NET_BIND_SERVICE, SETFCAP, SETGID, SETPCAP, SETUID, SYS_CHROOT`. Restricted
narrows `add` to at most `NET_BIND_SERVICE`.
‡ Baseline safe sysctls: `kernel.shm_rmid_forced`, `net.ipv4.ip_local_port_range`,
`net.ipv4.ip_unprivileged_port_start`, `net.ipv4.tcp_syncookies`,
`net.ipv4.ping_group_range`, `net.ipv4.ip_local_reserved_ports`,
`net.ipv4.tcp_{keepalive_time,fin_timeout,keepalive_intvl,keepalive_probes}`.
§ Restricted-allowed volume types: `configMap, csi, downwardAPI, emptyDir, ephemeral,
persistentVolumeClaim, projected, secret`.

A Baseline-only clause scans the Baseline rows; a Restricted clause scans all rows.
The distinction is a set choice, not a code path.

### 2.2 RBAC good-practices — authorize controls

Scanned on `kind ∈ {Role, ClusterRole, RoleBinding, ClusterRoleBinding}`. All
contribute the **authorize** facet.

| control | where | violating shape |
|---|---|---|
| cluster-admin binding | `roleRef.name` on a `ClusterRoleBinding` | `cluster-admin` (or any binding whose `subjects` include `system:masters`) |
| wildcard rule | `rules[*].{verbs,resources,apiGroups}` | contains `"*"` |
| escalation verbs | `rules[*].verbs` with `resources ⊇ {roles, clusterroles}` (`rbac.authorization.k8s.io`) | `escalate`, `bind` |
| impersonation | `rules[*].verbs` = `impersonate` on `resources ∈ {users, groups, serviceaccounts}` (core) | present |
| webhook config write | `create`/`update`/`patch` on `validatingwebhookconfigurations`, `mutatingwebhookconfigurations` (`admissionregistration.k8s.io`) | present |
| node proxy | any verb on `nodes/proxy` (`get` is not read-only here) | present |
| pod exec/attach/pf | `create`/`get` on `pods/exec`, `pods/attach`, `pods/portforward` | present → also **exec** facet |
| ephemeral containers | `create`/`update` on `pods/ephemeralcontainers` | present → also **exec** facet |
| CSR issue/approve | `create`/`update` on `certificatesigningrequests`, `…/approval` (`certificates.k8s.io`) | present |
| SA token mint | `create` on `serviceaccounts/token` (core) | present → also **secret** facet |
| PV create | `create` on `persistentvolumes` (core) | present (can smuggle `hostPath`) → also **locus** |
| secret read | `get`/`list`/`watch` on `secrets` (core) | present → **secret** facet |
| workload create | `create` on `pods`, `deployments`/`daemonsets`/`statefulsets`/`replicasets` (`apps`), `jobs`/`cronjobs` (`batch`), `replicationcontrollers` (core) | present → run-as-SA escalation vector |

### 2.3 Non-workload kinds

| kind | facet | note |
|---|---|---|
| `Secret` | secret | credential material in the manifest (or referenced) |
| `Namespace` | authorize/locus | create/relabel changes PSA enforcement scope |
| `CustomResourceDefinition` | authorize | extends the API surface; unknown downstream semantics |
| `NetworkPolicy` | locus | relevant on delete/replace-shrink, not create; flagged for level policy |
| `ValidatingWebhookConfiguration` / `MutatingWebhookConfiguration` | authorize/exec | intercepts all admission; §2.2 covers the RBAC to write them, this covers the object itself |

`kind: List` (`v1`) and `kind: <Anything>List` are unwrapped into their `items[*]` and
each scanned (§3).

## 3. Parse strategy & boundedness

**Bounded by construction.** The resolver is a **shallow structured-field scan over a
fixed, finite set of dotted paths** after a real parse. It performs no semantic
evaluation — no reference resolution, no RBAC transitive closure, no expression
evaluation. The path set is closed (§2); adding a control is adding a row, not
widening the algorithm. Cost is `O(documents × paths)`.

**Pipeline.**

1. **Decode.** Parse the buffer with a real YAML parser (YAML 1.1/1.2 superset of
   JSON — one parser covers both). Split multi-document streams on `---`. A decode
   error on **any** document → whole set `Deny`.
2. **Resolve aliases/merges before scanning.** YAML anchors `&x` / aliases `*x` and
   merge keys `<<:` can move a dangerous field out of the literal scan path. The
   parser must produce a fully-materialized node tree (aliases expanded, merge keys
   applied) before any path lookup. Scanning raw pre-merge text is unsound.
3. **Unwrap lists.** `kind: List` / `*List` → scan each `items[*]` as a document.
4. **Recurse into embedded pod templates.** The PSS paths (§2.1) live at the *pod
   spec*, which is nested under workload kinds. Fixed recursion table:

   | kind | pod-spec path |
   |---|---|
   | `Pod` | `spec` |
   | `Deployment`, `ReplicaSet`, `DaemonSet`, `StatefulSet`, `ReplicationController` | `spec.template.spec` |
   | `Job` | `spec.template.spec` |
   | `CronJob` | `spec.jobTemplate.spec.template.spec` |
   | `PodTemplate` | `template.spec` |

   A workload kind whose pod-spec path is absent/malformed → `Deny` (can't prove the
   template safe).
5. **Scan** the applicable §2 paths at each located spec / RBAC object; collect
   `controls` and `facets`.
6. **Verdict** per §1.

**Render-first sources are worst-case.** `-k` (kustomize) and helm charts are *not*
the manifest — they are programs that emit manifests. Rendering them (`kustomize
build`, `helm template`) is itself execution and, for helm, can pull remote charts
and run template functions; a chart may also require network to fetch dependencies.
The resolver does **not** invoke a renderer. An unrendered kustomize/helm source is
treated as `unknown` integrity at gate 2 and never reaches R3 as an admit path. If a
level wants helm/kustomize, the render must happen upstream under a separate,
explicitly-trusted capability whose *output file* then re-enters as a `worktree`
manifest.

**Gotcha checklist (all must be handled or they are silent bypasses):**

- multi-doc stream — scan every doc, not just the first;
- JSON input — same parser, same paths;
- anchors/aliases/merge keys — materialize before scan (step 2);
- `kind: List` wrapping — unwrap (step 3);
- embedded pod templates — recurse (step 4);
- unknown/future `kind` or `apiVersion` — `Deny` (§5), never scan-and-pass;
- pod-level vs container-level `securityContext` — a container field can override a
  safe pod default; scan both and take the violating one.

## 4. Output contract & level composition

R3 feeds two consumable shapes on the `R3Result`: the `controls` flag set (a
`payload_deny` list matches against it) and the `kinds`/`facets` sets (an
allowed-kinds gate matches against them). Level clauses stay in the v1.3 §4.1 TOML
idiom; the resolver is invoked by `delegate = "payload"` with an R3 predicate
present.

**Example — developer clause (dev-namespace benign workloads):**

```toml
[[level.developer.allow]]              # kubectl apply of benign dev workloads
delegate     = "payload"               # engages the k8s R3 resolver
verb         = ["apply", "create", "diff"]                       # gate 1
source       = "<= worktree"                                     # gate 2: in-repo file only
selector     = { kind = ["Deployment","Service","ConfigMap","Job","CronJob"],
                 namespace = ["dev","staging"] }                 # gate 3 + allowed-kinds
payload_profile = "restricted"          # scan Baseline ∪ Restricted (§2.1)
payload_deny    = ["*"]                 # any PSS control present → deny the clause
```

`kind = [...]` excludes `Secret`, `ClusterRoleBinding`, `CustomResourceDefinition` by
omission (allowlist floor). `payload_deny = ["*"]` denies on *any* §2 control — the
strictest and simplest form; a Deployment with `privileged: true` or `hostPath`
fails.

**Example — platform clause (allows Baseline, still forbids escalation):**

```toml
[[level.platform.allow]]
delegate     = "payload"
verb         = ["apply", "create"]
source       = "<= worktree-trusted"                             # hash-pinned trusted root
selector     = { kind = ["Deployment","DaemonSet","Role","RoleBinding"],
                 namespace = ["kube-system","platform"] }
payload_profile = "baseline"            # host namespaces/hostPath/privileged still denied
payload_deny    = ["*"]                 # incl. all RBAC authorize controls (§2.2)
```

A `RoleBinding` is admitted structurally, but a rule with `verbs: ["*"]`, `escalate`,
or a `cluster-admin` `roleRef` trips a §2.2 control and denies the clause.
`ClusterRoleBinding` is excluded from `kind` entirely.

Everything not matched by a clause stays denied. The resolver only ever *describes
more of the safe region precisely* — it cannot widen the floor.

## 5. Failure & worst-case

The R3 clause **denies** — never guesses admissible — on every
incomplete-information condition. This is the allowlist floor (`…-payload-frame`
§3.5): a clause the resolver cannot fully satisfy simply fails.

| condition | verdict | why |
|---|---|---|
| unparseable YAML/JSON (any doc in the set) | Deny | cannot prove the set safe |
| unknown / future `kind` or `apiVersion` | Deny | no scan spec ⇒ danger unprovable |
| known kind, malformed/absent expected spec path | Deny | template not verifiable |
| a control field present but unevaluable (unexpected type/shape) | Deny | treat as violating |
| source is `-f -` (stdin), `-f https://`, or `--raw` | Deny at R1 (never enters R3) | gate 2 integrity floor |
| kustomize/helm source not pre-rendered | Deny (unknown integrity) | rendering is execution / network |
| a language with no R3 resolver, under an R3 clause | Deny | clause unsatisfiable |

A pod spec with **zero** dangerous controls and an all-known-kinds document set is the
only admit path. Absence of a scanned field is safe **only** where the PSS "allowed
value" set includes `undefined/nil` (most Baseline controls); Restricted controls that
*require* an explicit safe value (`seccompProfile.type`, `capabilities.drop: [ALL]`,
`runAsNonRoot: true`) treat absence as a violation, matching upstream PSA.

## 6. Boundaries & non-goals

- **Not admission control.** It does not replace Pod Security Admission, and it runs
  client-side before apply, not in the apiserver path.
- **Not a policy engine.** It is not OPA/Gatekeeper/Kyverno; it evaluates a fixed
  control set, not user-authored policy.
- **Not a linter.** It does not check schema validity, best practices, or correctness
  beyond the §2 danger paths.
- **No RBAC transitivity.** It flags a rule's *shape* (`verbs: ["*"]`, `escalate`,
  cluster-admin `roleRef`); it does **not** compute what a binding transitively
  grants, resolve `roleRef` to its `ClusterRole`, or reason across documents.
  Cross-document reference resolution is out of scope beyond the fixed dotted paths.
- **Ambient target locus stays HP-12.** Which cluster the manifest lands on is set by
  kube-context / `$KUBECONFIG`, invisible to the resolver. R3 tightens the *content*
  axis only; the dominant *which-remote* facet is unaddressed here by design.

## Findings

- **k8s R3 is tractable because the danger set is externally specified and finite.**
  PSS (Baseline ⊂ Restricted) and RBAC good-practices give a ready-made, versioned
  control list; the resolver is a fixed dotted-path scan, not a semantic evaluator.
  Boundedness is structural, not a budget.
- **The two genuinely hard parts are render-first sources and RBAC transitivity.**
  helm/kustomize *are programs*, so sound R3 over them requires execution the resolver
  must refuse — push rendering to a separate trusted capability and re-enter its
  output as a `worktree` file. RBAC's real risk is transitive (what a `roleRef`
  ultimately grants), which the resolver deliberately does **not** compute; it flags
  rule *shape* only and worst-cases the rest.
- **Correctness hinges on three easily-missed preprocessing steps:** materialize
  anchors/aliases/merge keys before scanning, unwrap `kind: List`, and recurse into
  embedded pod templates (`spec.template.spec`, `spec.jobTemplate.spec.template.spec`).
  Skipping any is a silent bypass, not a visible error.
- **v1 scope recommendation — ship the Baseline PSS controls plus a small RBAC-shape
  set, `payload_deny = ["*"]` only.** Concretely: all §2.1 **Baseline** rows
  (privileged, host namespaces, hostPath, hostPorts, added-caps, procMount, unconfined
  seccomp, HostProcess) with recursion into workload pod templates; plus the
  highest-signal §2.2 rows (`cluster-admin` binding, wildcard rule,
  `escalate`/`bind`/`impersonate`, webhook-config write, `nodes/proxy`, `pods/exec`);
  plus `kind`-exclusion of `Secret`/`ClusterRoleBinding`/`CRD`. Defer the full
  Restricted profile (require-explicit-value controls: seccomp-required, drop-ALL,
  runAsNonRoot) and fine-grained per-control `payload_deny` lists to v2 — `["*"]` on
  the Baseline set already denies every real escape vector while keeping the v1 verdict
  logic a single "any control present" test.

Sources: [Pod Security Standards](https://kubernetes.io/docs/concepts/security/pod-security-standards/),
[RBAC good practices](https://kubernetes.io/docs/concepts/security/rbac-good-practices/).
