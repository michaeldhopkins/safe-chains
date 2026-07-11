# Harness behaviors

A maintainer reference for how each agent harness drives its pre-execution
("PreToolUse") hook: what envelope it sends us, what decision contract it
expects back, what reaches the model, and the per-target quirks that make wiring
easy to get silently wrong. The authoritative wiring lives in `src/targets/*.rs`;
this file is the consolidated picture so we don't have to re-derive it from
scattered comments each time.

When a fact here disagrees with a harness's current docs, the harness wins —
update both the relevant `targets/*.rs` and this file.

## The model we rely on

safe-chains is **allowlist-only**; what it EMITS for a given command depends on the
**harness's capabilities** (`docs/design/harness-capability-model.md`):

- **Safe command** → `allow` where the harness supports granting (Claude), else *silent* (the
  command just runs — e.g. Codex, which has no grant).
- **Gated command** → *silent* where a human still reviews it (Claude's own prompt is the check),
  else **`deny`** where the harness offers no interactive approval and no `ask` (Codex: staying
  silent would just run it, and its sandbox permits broad reads). Exceptions on a deny-harness are
  config-level (a custom command / grant), not per-invocation.

So we *do* actively deny on some harnesses (Codex) — the flip from the old "never deny" is
deliberate and capability-driven; see the design doc + the per-target table below.

Two consequences that shape everything else:

- **A hook firing does not pause the world.** PreToolUse runs before the tool,
  but our abstain (or a non-decision context injection) does not block. The
  command proceeds through the normal approval path and, if approved, runs. By
  the time any model-visible note we emit is read, **the command has most likely
  already run.** Feedback we inject must therefore be guidance for *next time*,
  never an instruction to re-run. (See `cst/explain.rs::guidance`.)
- **Context injection must not carry a permission decision.** To surface a note
  to the model while leaving approval to the normal flow, the envelope must omit
  the decision field. Including one (even "ask") overrides the user's allowlist.

## Decision contract per target

The field name and accepted values differ per harness. Getting the field name
wrong fails *silently* — the harness ignores the unknown key and falls back to
its own permissions, so a mis-wired target looks like it "works" (commands still
run) while never actually auto-approving. Tests assert the exact field.

| Target  | Tool / matcher          | Envelope shape                         | Decision field      | Values honored        | Timeout unit |
|---------|-------------------------|----------------------------------------|---------------------|-----------------------|--------------|
| Claude  | `Bash`                  | `{hookSpecificOutput: {...}}`          | `permissionDecision`| allow / deny / ask / (omit = defer) | — |
| Codex   | `Bash`; config nests under top-level `hooks` | `{hookSpecificOutput: {...}}` | `permissionDecision`| **deny** (gated); *silent* (safe). `allow`/`ask` parsed-but-UNSUPPORTED on v0.144.3 | — |
| Qwen    | `^Bash$`                | `{hookSpecificOutput: {...}}` (mirrors Claude) | `permissionDecision` | allow          | ms |
| Droid   | `Execute`               | `{hookSpecificOutput: {...}}` (mirrors Claude) | `permissionDecision` | allow          | seconds |
| Gemini  | `^run_shell_command$`   | `{decision: ...}`                      | `decision`          | allow / deny **only** | ms |
| Cursor  | (settings `version: 1`) | flat object                            | `permission`        | (cursor's set)        | — |
| Copilot | (no matcher; self-filter on `toolName`) | flat object, `toolArgs` is a nested JSON string | `permissionDecision` | **only `deny` is currently processed** | — |

Notes:
- **Droid**'s shell tool is `Execute`, not `Bash` — a `Bash` matcher never fires.
- **Gemini/Cursor** use `decision` / `permission`, *not* `permissionDecision`.
- **Copilot** today only acts on `deny`; for a safe command the honest answer is
  "no opinion" (empty), letting its allow-by-default apply. We still emit an
  allow-shaped envelope so future Copilot releases that honor allow can use it.

### Codex — researched 2026-07-13, v0.144.3 (probe-verified)

- **Official docs:** https://learn.chatgpt.com/docs/hooks · https://developers.openai.com/codex/concepts/sandboxing · https://developers.openai.com/codex/agent-approvals-security
- **Config schema:** `~/.codex/hooks.json` must nest events under a top-level `hooks` object:
  `{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"safe-chains hook codex"}]}]}}`.
  A flat top-level `PreToolUse` (Claude's shape) is **rejected** — `unknown field PreToolUse` — and the
  hook never loads. Codex prompts to *trust* a new/changed hook on first run.
- **Input:** `tool_input.command` (Bash), `cwd`, plus `tool_name`/`model`/`permission_mode`/… (ignored).
- **Decision values:** `deny` and (per docs) `allow`; but **`allow` errored on v0.144.3**
  (`unsupported permissionDecision:allow`) — version drift. `ask` / legacy `approve` / `continue:false`
  / `stopReason` / `suppressOutput` are **parsed but not supported**. An unsupported decision → Codex
  "marks the hook failed, reports the error, and **continues the tool call**" (fail-open — emit only a
  supported shape).
- **Capabilities:** grant ❌ (allow unsupported here) · deny ✅ · escalate/`ask` ❌ · human-review-on-silence ❌
  (only sandbox-*escape* prompts) · sandbox ✅ `workspace-write` (blocks out-of-workspace writes + network,
  **permits broad reads**). → safe-chains emits **silent** for safe, **`deny`** for gated (`src/targets/codex.rs`).
- **Verified end-to-end:** safe `cat README.md` runs clean; gated `cat /etc/hosts` (sandbox would allow
  the read) is **blocked by the hook** with the config-exception reason.

### Gemini → Antigravity — researched 2026-07-13

- **Gemini CLI is RETIRED (2026-06-18)** for AI Pro/Ultra/free-tier users; it survives only under a
  Gemini Code Assist Standard/Enterprise license. Our `gemini` target is therefore **DEPRECATED** —
  keep it (enterprise remnant, can't become unsafe) but drop it from the active test matrix. Note:
  the old `gemini` row above modeled a stdout `{decision}` shape, but Gemini actually blocked via
  **shell exit codes** — an unverified assumption, now moot. Sources:
  https://developers.googleblog.com/an-important-update-transitioning-gemini-cli-to-antigravity-cli/
- **Successor: Antigravity CLI, command `agy`** (closed-source Go binary, multi-agent). It HAS
  PreToolUse hooks. Config via `hooks.json` in a customization root — `.agents/hooks.json` per
  project, `~/.gemini/config/hooks.json` globally (both merged). Top-level keys are hook *names*
  (`safe-chains`), each mapping to `{ "PreToolUse": [ { "matcher": "...", "hooks": [ { "type":
  "command", "command": "..." } ] } ] }`. `/hooks` inspects active hooks. Sources:
  https://medium.com/google-cloud/a-developers-guide-to-agent-hooks-in-antigravity-cli-4c1440febd11 ·
  the bundled `~/.gemini/antigravity-cli/builtin/skills/agy-customizations/docs/hooks.md`.

#### Antigravity CLI (`agy`) — VERIFIED LIVE, v1.1.2, 2026-07-13

Driven live via the TUI harness in `~/projects/safe-chains-harness-lab` (Gemini 3.5 Flash).

- **Matcher target = the tool NAME `run_command`** (the shell tool). The UI/log label it "Bash", but
  the payload's `toolCall.name` is `run_command`, and the matcher matches that. Verified: matcher
  `run_command` fires (log `jsonhook.go:189 Loaded hooks.json … 1 named hooks` immediately before the
  confirmation). `"*"` / `""` also match all tools.
- **Input (stdin, protojson/camelCase):** shell command at `toolCall.args.CommandLine`; workspace at
  `workspacePaths[0]` (used as both cwd and root). Also carries `conversationId`, `stepIdx`,
  `modelName`, `transcriptPath`.
- **Output (stdout):** `{ "decision": <d>, "reason": <str>, "permissionOverrides": [...] }`.
  Decisions and their **observed** effect on v1.1.2 CLI:
  - `"deny"` → **hard block**; the model is told "Tool call denied by pre-tool hook: <reason>". WORKS
    (verified: `cat /etc/hosts` blocked, our reason shown).
  - `"force_ask"` → **forces a human prompt**, ignoring the Always-Allow cache. WORKS (verified:
    `cat /etc/hosts` surfaced the permission prompt).
  - `"ask"` → prompts, but **respects the Always-Allow cache** (a prior "always allow commands
    starting with cat" would auto-run a gated `cat …`). We use `force_ask` instead to close that hole.
  - `"allow"` → does **NOT** suppress agy's own confirmation in the CLI. Verified twice: `decision:
    allow` (with and without `permissionOverrides:["command(cat README.md)"]`) still surfaced the
    prompt. **So there is no effective GRANT on agy 1.1.2** — a safe command can't skip the prompt.
    We still emit `allow` (semantically correct, harmless, future-proof).
  - `"permissionOverrides"` → does **NOT** register a session grant either. Tested with a stateful
    hook: call 1 emitted `{allow, permissionOverrides:["command(echo hello)"]}`; call 2 (same command)
    emitted a cache-respecting `ask` — and **still prompted**. So the hook has NO way, current-call or
    future-call, to make a safe command auto-run. agy's own native allowlist ("always allow …",
    persisted to settings.json) and permission modes are the ONLY auto-approval paths, and they are
    user-driven — safe-chains can't feed them from the hook.
- **Human review on silence: YES.** Default setting `toolPermission=request-review` prompts for
  `run_command`. So agy has per-command human review even with no hook decision.
- **Fails CLOSED** on a missing/malformed decision (prompts rather than runs) — the opposite of
  Codex. Every response therefore carries an explicit `decision`.
- **Capability summary:** grant ❌ · deny ✅ · escalate/`ask`+`force_ask` ✅ · human-review-on-silence
  ✅ · fail-closed ✅. → gated command policy = **Ask** (escalate via `force_ask`), not Deny: agy has
  human review, so we escalate rather than hard-block. See `src/targets/agy.rs`.

## Model-visible context injection (`additionalContext`)

This is what powers the chain-explainer feedback (`cst::explain`). It injects
text into the model's context **without** a permission decision, so the normal
flow is untouched.

| Target  | `additionalContext` support | What we do |
|---------|-----------------------------|------------|
| Claude  | Yes (verified — Claude Code hooks docs) | emit on the mixed-chain case |
| Qwen    | Assumed (declares Claude-Code-mirror)   | emit on the mixed-chain case |
| Droid   | Assumed (declares Claude-Code-mirror)   | emit on the mixed-chain case |
| Codex   | Unverified                  | default abstain (emit nothing) |
| Cursor  | Unverified / different shape | default abstain |
| Gemini  | Unverified (decision-only contract) | default abstain |
| Copilot | Unverified / flat, deny-only | default abstain |

Targets without verified support keep `HookFormat::render_context`'s default,
which emits an empty body — i.e. exactly the prior abstain behavior, no
regression. Promote a target out of the default only after confirming its hook
schema has a context field that does not change the permission outcome.

Source of the verified Claude facts: Claude Code hooks documentation
(`hookSpecificOutput.additionalContext`, `permissionDecision` values, and that
`ask`/`deny` override the user's allowlist while `additionalContext` alone does
not).

## Hook payload capabilities — what each harness can pass

We currently parse only `command` (+ `cwd`, into `HookInput.cwd`, unused so far).
But the harnesses hand a hook far more, and the **project root** in particular is
what an accurate cwd-aware classifier needs (HP-19): resolving `cwd + relative`
only tells "in the project" from "in /etc" if we know the root, and safe-chains
must never touch the filesystem to find it, so it has to arrive as input.

**Working-directory / project-root availability** (⚠️ reverse-engineered from
harness docs + our own parsers; not yet verified against live harnesses — see the
e2e note):

| Harness  | `cwd` (payload) | project root | root arrives via |
|----------|:---------------:|:------------:|------------------|
| Claude   | ✅ | ✅ | `CLAUDE_PROJECT_DIR` env var (hook process) |
| Cursor   | ✅ | ✅ | `workspace_roots[]` in the payload JSON |
| Gemini   | ✅ (also `GEMINI_CWD` env) | ✅ | `GEMINI_PROJECT_DIR` env var |
| Droid    | ✅ | ✅ | `FACTORY_PROJECT_DIR` env var |
| Qwen     | ✅ | ✅ | `QWEN_PROJECT_DIR` env var |
| Codex    | ✅ | ❌ | cwd only — no distinct root (`cwd` is the "project dir equivalent") |
| Copilot  | ✅ | ❌ | cwd only — no documented root field/var |
| opencode | ❌ | ❌ | not a runtime hook — we render static config patterns; no per-command signal |

So **5 of 8 give a distinct project root** (env var for 4, payload array for
cursor); Codex/Copilot give `cwd` but no root; opencode gives nothing at eval
time. When a root is unavailable, the classifier must fall back to today's
`relative == worktree` assumption — use the signal where present, never regress.
Env-var roots are read from the hook *process* env (set by the harness, not the
agent's shell), so they are reasonably tamper-resistant.

**The full rubric — everything a harness may pass** (union; no harness sends all).
Kept as a menu for *future* safe-chains features, with the plausible use:

| Field (varies in name/casing) | Harnesses | Possible safe-chains use |
|---|---|---|
| `command` / `tool_input.command` | all | the thing we classify (why we exist) |
| `tool_name` | Claude, Gemini, Codex, Droid, Copilot | scope the hook to Bash; skip non-shell tools |
| `cwd` | all but opencode | cwd-aware classification (HP-19 #1) |
| project root (env / `workspace_roots`) | Claude, Cursor, Gemini, Droid, Qwen | the root HP-19 needs |
| `session_id` / `conversation_id` | all but opencode | **unforgeable session key for HP-17** session grants |
| `transcript_path` | Claude, Gemini, Codex, Droid, Qwen | richer context; audit trail |
| `hook_event_name` | most | confirm we're on the pre-exec event |
| `tool_use_id` / `turn_id` / `generation_id` | Claude, Codex, Cursor | correlate a decision to a specific call |
| `model` | Codex, Cursor | per-model policy |
| `permission_mode` | Claude, Droid | respect the user's current mode |
| `timestamp` | Gemini, Qwen, Copilot | logging |
| `cursor_version` / cli version | Cursor | version-gate quirks |
| `user_email` | Cursor | attribution |
| `last_assistant_message` / `input_messages` | Codex (legacy notify) | intent context |

**e2e testing (owed).** All of the above is derived from docs and our parsers, not
observed on this machine. The env-var roots especially (`CLAUDE_PROJECT_DIR`,
`GEMINI_PROJECT_DIR`, `FACTORY_PROJECT_DIR`, `QWEN_PROJECT_DIR`) must be confirmed
to actually be present in the hook *process* environment at runtime before we
depend on them. We should eventually stand up **end-to-end tests that install
safe-chains into each harness on this computer** and capture a real payload +
process env per harness — both to verify this table and to catch schema drift.
Until then, treat every ✅ here as "documented, unverified," and keep the
None-fallback so an absent field is never a security regression.
