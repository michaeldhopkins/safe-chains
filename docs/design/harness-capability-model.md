# Harness capability model — how safe-chains' emission adapts to each harness

**Status:** design draft (2026-07-13). Reacting artifact before touching the hook flow.

## The problem

safe-chains classifies a *command* two ways: **safe** (it auto-approves) or **gated** (it does
not). But what the hook should actually *emit* for a gated command is **not** a property of the
command — it's a property of the **harness**:

- In **Claude Code**, a silent hook falls back to Claude's own per-command **permission prompt**, so
  staying silent hands the decision to a human. Abstaining is safe; *denying* would be wrong (it
  would block commands the human would have approved).
- In **Codex**, there is **no per-command human prompt**. In-workspace commands just run; the human
  is asked only for *sandbox escapes* (writes outside the workspace, network). So a silent hook =
  the command **runs**. Worse, Codex's `workspace-write` sandbox permits **broad reads**, so
  `cat /etc/shadow` runs and discloses the secret to the model with no check at all. On Codex the
  hook must **deny** a gated command — safe-chains has to *be* the check.

One fixed emission strategy cannot serve both. This is the exact shape of the command model turned
on the harness: as **command → capabilities → level**, so **harness → capabilities → emission**.

## What safe-chains does for you, per harness

One row per harness we have verified. Add a row only after that harness is probed and implemented.

| Harness | How it handles safe commands | How it handles dangerous commands | Worth using safe-chains? |
|---|---|---|---|
| Claude Code (`claude`) | Auto-approves | No change | Yes, for fewer prompts overall |
| Codex (`codex`) | No change | Blocks them | Only if you want unsafe commands blocked |
| Antigravity (`agy`) | No change | Forces a prompt | Only if you want unsafe commands prompted |

Note: the agy row may reduce prompts on safe commands *for free* if a future agy build honors the
hook's `allow` (its docs already claim `allow` auto-allows; v1.1.2 CLI does not). safe-chains already
emits `allow` for safe commands, so no code change is needed if/when that lands.

## Harness capability facets

For each harness (at a given version, and sometimes a given mode) we determine, *by evidence*:

| Facet | Question | How we learn it |
|---|---|---|
| **grant** | Can a hook PRE-APPROVE a command, skipping the harness's own prompt? | probe: emit an allow-shaped response for a safe command; does it run without a prompt, or error? |
| **deny** | Can a hook VETO a command? | docs + probe: emit a deny; is the command blocked? |
| **escalate** | Can a hook FORCE the harness to ask the human right now (an `ask`/approve decision)? | docs + probe: emit an ask-shaped response; does a prompt appear, or is it ignored/unsupported? |
| **human-review-on-silence** | If the hook emits nothing, does a HUMAN still decide *this* command? | probe: emit nothing for a gated command; does the harness prompt, or just run it? |
| **sandbox** (context) | Is there a coarse backstop that limits blast radius regardless of the hook? | docs + probe: run an out-of-workspace write with a silent hook; blocked by a sandbox, or executed? |
| **continue-on-malformed** (safety note) | If the hook emits an UNSUPPORTED decision, does the harness run the command anyway? | docs + probe. If yes, our emission must be *exactly* the supported shape or we fail open. |

`sandbox` doesn't change the emission — it's a *second* layer we note so we understand residual
risk (e.g. Codex's sandbox stops out-of-workspace *writes* and network, but **not** broad reads).
`escalate` and `human-review-on-silence` both route a gated command to a *human*; either one is
enough to defer instead of deny.

## The emission principle

Given a command's classification and the harness's capabilities:

**Safe command** (want it to run, ideally without friction):
- **grant** present → emit `allow` (skip the harness prompt).
- else → emit **silent** (the command just runs; there was no prompt to skip anyway).

**Gated command** (must reach a check), in priority order:
- **escalate** present → emit `ask` — the human decides *in the moment*, no hard block, no config edit. Best case.
- else **human-review-on-silence** present → emit **silent** (the harness's own prompt is the check).
- else **deny** present → emit **deny** — safe-chains *is* the check; exceptions become config-level (see below).
- else → emit **silent** and document the gap (rely on the sandbox if one exists).

The load-bearing invariant: **a gated command never executes without *some* check** — a human (via
`escalate` or silence-to-prompt) where the harness offers one, else safe-chains' own deny. `grant`
is an orthogonal convenience (skip a prompt) that only applies where the harness supports it and
there is a prompt to skip.

**Consequence — this is why the Codex answer is `deny`, not silence.** Codex offers no
human-review-on-silence but does offer deny, so a gated command must be denied. The apparent
"over-block" (deny everything not on the allowlist) is the *correct posture for an autonomous
harness*: an unsupervised agent should be confined to the known-safe set, with the sandbox as a
second layer. It is the same tradeoff as running any agent unattended.

## Exceptions and overrides — how a human allows a gated command

Where a gated command is emitted as **`ask`** or deferred to a **silence-prompt**, the human simply
approves it in the moment. Where it is **denied** (a deny-only harness like Codex), there is no
in-the-moment approval — the veto is authoritative. Exceptions are then **config-level**, the
allowlist-native mechanism that works on every harness:

- **Custom command definition** — add the tool/subcommand/flags to `~/.config/safe-chains.toml`
  (or a project `.safe-chains.toml`); safe-chains reclassifies it as *safe* → allowed thereafter.
- **Directory grant** — for path-based cases, widen the workspace via a `[[grant]]`.

To allow more, you widen the allowlist — you don't approve a single invocation. This is consistent
with safe-chains being an allowlist; the in-the-moment prompt on an interactive harness is a
*convenience* a deny-only harness doesn't offer. So the "strict deny" posture always has an escape
valve (extend the config), it just isn't per-invocation.

## Per-harness classification (evidence-based)

Only rows with a **date** + **version** are verified by our own probe; the rest are assumptions to
confirm one harness at a time.

| Harness | grant | deny | escalate (`ask`) | human-review-on-silence | sandbox | → SAFE | → GATED | Evidence |
|---|---|---|---|---|---|---|---|---|
| **Claude Code** | ✅ | ✅ | ✅ | ✅ (permission prompt) | ❌ | `allow` | silent (defer to prompt) | reference; long-standing |
| **Codex** | ✅ per docs / ❌ v0.144.3 | ✅ | ❌ (`ask` parsed, unsupported) | ❌ (only sandbox-escape prompts) | ✅ workspace-write | **silent** | **deny** | probe 2026-07-13, v0.144.3 |
| ~~gemini~~ | — | — | — | — | — | — | — | **DEPRECATED**: Gemini CLI retired 2026-06-18 (enterprise-only remnant) |
| **antigravity** (`agy`) | ❌ (`allow` doesn't skip confirm) | ✅ | ✅ (`ask`/`force_ask`) | ✅ (`request-review`) | ❌ | `allow` | **`force_ask`** | probe 2026-07-13, v1.1.2 (live TUI) |
| qwen | ? | ? | ? | ? | ? | ? | ? | unverified |
| cursor | ? | ? | ? | ? | ? | ? | ? | unverified |
| copilot | ❌ (honors only deny) | ✅ | ? | ? | ? | ? | ? | doc-only; probe pending |
| droid | ? | ? | ? | ? | ? | ? | ? | unverified |
| opencode | ? | ? | ? | ? | ? | ? | ? | unverified |

**Codex evidence (v0.144.3, probed 2026-07-13):** supports `allow`/`deny` only; `ask`, legacy
`approve`, `continue:false`, `stopReason`, `suppressOutput` are *parsed but not supported*. An
unsupported decision → Codex "marks the hook run as failed, reports the error, and **continues the
tool call**" (a fail-open — our emission must be exactly a supported shape). `allow` errored on
v0.144.3 despite the current docs listing it as supported (version drift), so SAFE → **silent**
(Codex continues → runs) is the version-robust choice. Config must nest under a top-level `hooks`
object (not Claude's flat `PreToolUse`). Sandbox `workspace-write` blocks out-of-workspace writes +
network but permits **broad reads** — so a gated `cat /etc/shadow` runs unchecked unless the hook
denies it.

**Antigravity evidence (v1.1.2 CLI, probed 2026-07-13 live):** the `run_command` PreToolUse hook
reads protojson on stdin (`toolCall.args.CommandLine`, `workspacePaths`) and writes a `{decision}`.
Observed: `deny` hard-blocks (reason shown to the model); `force_ask` forces a human prompt;
`ask` prompts but respects the Always-Allow cache (a coarse "always allow cat" grant would auto-run
a gated `cat …`); **`allow` does NOT suppress agy's own `request-review` confirmation** — verified
twice, with and without `permissionOverrides`, so there is no effective grant on the CLI. Because
agy already prompts on silence (human review present), a gated command **escalates via `force_ask`**
(not a hard deny, unlike Codex) — and `force_ask` over `ask` closes the Always-Allow-cache hole.
SAFE still emits `allow` (harmless, future-proof). Fails CLOSED on a missing/malformed decision.

## Mapping to code

1. A per-target **`Capabilities { grant, deny, human_review_on_silence }`** (a method on `Target`,
   defaults conservative).
2. `main.rs` hook flow reads it:
   - safe verdict → `if grant { target.render_allow() } else { empty }`
   - gated verdict → `if human_review_on_silence { context/empty } else if deny { target.render_deny(reason) } else { empty }`
3. Each deny-capable target implements **`render_deny(reason)`** (Codex/Claude: the
   `hookSpecificOutput.permissionDecision:"deny"` shape).
4. The cross-target contract guard (`every_target_hook_contract_is_fail_safe`) is extended to assert
   each target's emitted shapes match its declared capabilities (e.g. a target with `grant=false`
   must NOT emit an `allow` decision).

## Research discipline

`HARNESS-BEHAVIORS.md` becomes proper, dated research — per harness:
- **official docs URL(s)**,
- **version researched**, **date researched** (mirroring `researched_version` on commands),
- the four capability facts, each tagged *doc* vs *probe-verified*.

We re-research periodically and **re-verify with our own probes** (kept in the harness lab), because
harness hook contracts change (Codex's config schema already did — it rejected the Claude-shaped
flat `PreToolUse`).

## Modes, and respecting an explicit opt-out

Capabilities are stated per-harness above, but some are really per-**mode**, and the mode arrives in
the hook payload (`permission_mode` for Codex, similar elsewhere). Two rules:

- **Respect an explicit full-YOLO opt-out.** A user who runs Claude with
  `--dangerously-skip-permissions` has deliberately turned off all checks — safe-chains must not
  fight that. It already doesn't: on Claude we never deny, so normal mode gets the human prompt and
  YOLO mode auto-runs, which is exactly what that user chose. No change.
- **Comport to a "bounded-auto" mode.** A mode that is autonomous *but expects limits* (Codex's
  default: auto-run inside a sandbox) is where `deny`-on-gated belongs — the user installed
  safe-chains precisely to bound it. Codex has no non-bounded mode, so it always denies gated.

So mode-awareness is opt-in, per-harness, learned by evidence: we only read the mode when a harness
actually has a mode that flips a capability (a YOLO flag on a gating harness → stand down; a
bounded-auto mode on a prompting harness → gate). Neither Claude nor Codex needs it today.

## Workflow

Finish **one harness end-to-end** — research → probe → classify → implement → re-verify — before
adding the next. Keep each logged-in harness around so future changes can be re-checked across all
of them **quickly and cheaply**. Current state: **Claude** (reference, correct), **Codex**
(classified + `deny`-on-gated, verified v0.144.3), and **Antigravity `agy`** (classified +
`force_ask`-on-gated, verified live v1.1.2) are done. (`gemini` is deprecated — Gemini CLI retired
2026-06-18.) **Next: one of** qwen / cursor / copilot / droid / opencode, added one at a time.

## Reason enrichment on `Deny`/`Ask` harnesses

The rich workspace-overreach explanation ("reaches `~/.ssh/id_rsa`, a credential store …") reaches
`Defer` harnesses via `render_context` on the fall-through. `Deny`/`Ask` harnesses exit the
`gated_policy()` match early, so `main.rs` computes `workspace_overreach` **before** the match and
folds its "why" clause into the `render_deny`/`render_ask` reason when present — a gated
`cat ~/.ssh/id_rsa` on agy now `force_ask`s with the credential alarm, and on Codex the `deny` reason
names the out-of-workspace path, instead of the generic "not on the allowlist" text. Non-overreach
gated commands still get the generic reason. Covered by `codex_hook_gated_overreach_reason_names_the_path`
and the `antigravity_hook_*` tests in `tests/integration_hooks.rs`.
