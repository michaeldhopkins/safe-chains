# The agent scratchpad as a trusted working area

*Status: implemented (Claude Code), 2026-07-24. Extends the executor-origin band of
`docs/design/behavioral-taxonomy-execution-origin.md`.*

## 1. The problem

Agent harnesses give each session a **scratchpad** — a per-session temp directory where the agent
stages its own work: a generated script, an extracted archive, intermediate data. It is a first-class
part of how agents operate.

safe-chains classified it as plain `temp`, which made it read- and write-able but **not
executable**:

```
echo x > <scratchpad>/gen.py     ✓ allowed
cat      <scratchpad>/gen.py     ✓ allowed
python3  <scratchpad>/gen.py     ✗ DENIED
bash     ./build.sh              ✓ allowed  (worktree)
```

The denial is not a bug in the execute rule. `temp` sits **below** the execute clause's
`>= sandbox-scope` floor, and that floor is correct: `/tmp/x.sh` is exactly where a *downloaded*
script lands, and running foreign code is the thing the band exists to prevent. The problem is that
the agent's **own** working directory was indistinguishable from anonymous `/tmp`.

## 2. What the harness can tell us

Researched against Claude Code (2026-07):

- The PreToolUse envelope carries `session_id`, `prompt_id`, `transcript_path`, `cwd`,
  `permission_mode`, `effort`, `hook_event_name`, `tool_name`, `tool_input`, and (in subagents)
  `agent_id`/`agent_type`.
- Env vars: `CLAUDE_PROJECT_DIR`, `CLAUDE_PLUGIN_ROOT`, `CLAUDE_PLUGIN_DATA`, `CLAUDE_EFFORT`,
  `CLAUDE_CODE_REMOTE`, `CLAUDE_CODE_BRIDGE_SESSION_ID`.
- **The scratchpad path is NOT exposed** — no field, no env var. A feature request for a
  `CLAUDE_SCRATCHPAD_DIR` (anthropics/claude-code#45745) is marked *not planned*. The observed path
  (`/private/tmp/claude-<uid>/<project-slug>/<session-id>/scratchpad`) is internal and undocumented.

So the harness will not hand us the directory. But it does hand us the **session id**.

## 3. The mechanism: anchor on the session id, not the layout

> A path **under a temp root** that contains the **session id as a whole path component** is this
> session's scratchpad, and earns `sandbox-scope`.

Both halves are load-bearing:

- **The session id, as a component.** It arrives in the harness's own envelope, never from the
  agent's shell, so it is **unforgeable**: nobody can pre-plant `/tmp/<session-id>/evil.sh` because
  the id does not exist until the session does. Matching a whole *component* (not a substring) closes
  the prefix/suffix dodge (`/tmp/<id>-evil/`). Contrast a layout pattern like "anything under
  `/tmp/claude-*`", which anyone can create.
- **The temp-root requirement.** Keeps the id from blessing a path outside the scratch area on a
  harness that happens to embed the session id elsewhere (a log under `$HOME`, say).

Anchoring on the id rather than the layout also makes it **durable**: it survives the harness
reorganizing the parts *around* the id — the uid suffix, the slug, `/tmp` vs `/private/tmp`, the
trailing directory name — all of which are undocumented internals.

**It fails closed in every degenerate case.** No session id, a too-short/odd id, a different
session's id, or a path without the id → no recognition, and the path keeps its ordinary
classification. A harness that supplies nothing is exactly as restricted as before, never worse.

## 4. Why `sandbox-scope` specifically

The level model already reserved the rung. `developer`'s execute clause is an interior band —
`locus.local = ">= sandbox-scope, <= worktree-trusted"` — meaning "code that lives in your workspace,
not foreign code below it (`temp`, `process`) and not system code above it (`user`, `machine`)".
Until now **no path mapped to `sandbox-scope`**; it was a reserved rung awaiting exactly this case:
*a trusted working area that is not the worktree.*

So the scratchpad becomes runnable while every other temp path stays foreign. No level was
loosened, no rung invented — an existing rung got its first occupant.

## 5. Implementation

- `HookInput.session_id` (`src/targets/mod.rs`) — parsed from Claude's `session_id`. Every other
  target passes `None` until its scratchpad layout is researched (§7).
- `PathCtx.session_id` + `pathctx::in_session_scratchpad()` — the recognition rule. Carried through
  `enter_cwd` so an intra-line `cd` cannot drop recognition mid-chain.
- `regions::scratchpad_role()` — runs before the region table so the scratchpad is not first
  captured by the generic `/tmp` node; returns `sandbox-scope` for both faces.
- `--session-id` CLI flag — lets the whole path be exercised without a live harness.

Guards (`src/pathctx.rs`): `only_this_sessions_scratchpad_is_recognized` enumerates the spoofing
class (another session's id, substring/prefix/suffix dodges, right-id-wrong-root, anonymous temp) and
the legitimate spellings; `a_missing_or_unusable_session_id_recognizes_nothing` covers the
fail-closed cases. Both were verified red→green by weakening each of the two conditions in turn.

## 6. The nudge

A denial here is the one case whose remedy is usually "that IS my working directory", so
`ReachReason::ForeignTemp` explains it rather than silently prompting:

> it runs code from `/tmp/downloaded.sh`, a temporary directory that is not this session's
> scratchpad. Temp files can be read and written freely, but code there is treated as FOREIGN (a
> downloaded script lands in the same place), so running it is not auto-approved. If this is a
> working directory you trust, grant it in ~/.config/safe-chains.toml; a scratchpad the harness
> reports for this session is recognized automatically and needs no grant.

Detection is separate from the ordinary overreach check: a temp path is read/write **admitted**, so
the outside-workspace test never fires on it — the denial is specifically about *execution*.

## 7. Other harnesses — open work

`session_id`/`conversation_id` is present in every harness envelope except opencode, and
`HARNESS-BEHAVIORS.md` already treats it as "unforgeable session key". The remaining work per harness
is **not** the mechanism (it is harness-agnostic) but the research question:

> Does this harness stage per-session work in a temp directory, and does that path embed the session
> key?

| harness | session field | scratchpad layout |
|---|---|---|
| Claude | `session_id` | ✅ researched — id is a path component |
| Codex, Cursor, Gemini, Qwen, Droid, Grok | present in envelope | ⬜ not yet researched |
| opencode | none | n/a — config grant only |

Wiring one up is: parse its session field into `HookInput.session_id`, confirm the layout embeds it,
add a guard. Where a harness's scratchpad does *not* embed the session key, the fallback is the
config grant plus the §6 nudge — which is why the nudge matters beyond Claude.
