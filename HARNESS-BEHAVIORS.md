# Harness behaviors

A maintainer reference for how each agent harness drives its pre-execution
("PreToolUse") hook: what envelope it sends us, what decision contract it
expects back, what reaches the model, and the per-target quirks that make wiring
easy to get silently wrong. The authoritative wiring lives in `src/targets/*.rs`;
this file is the consolidated picture so we don't have to re-derive it from
scattered comments each time.

When a fact here disagrees with a harness's current docs, the harness wins —
update both the relevant `targets/*.rs` and this file.

## Verification scorecard (single source of truth)

The one place the per-harness status is collected — update it whenever a target is tested or its
support changes, so the status is never re-derived from the scattered sections below. "Verified live"
= driven end-to-end against the running harness; "probe-verified" = confirmed by sending crafted
envelopes to the harness's hook runner; "assumed" = declares a known-shape mirror but not
independently exercised.

| Harness  | Runtime hook | Status | Evidence / note |
|----------|:------------:|--------|-----------------|
| **Claude** | ✅ | ✅ **Verified live** (2026-07) + docs | Dogfooded — the allow envelope auto-approves, abstain→`additionalContext` surfaces the reason without blocking (this session). |
| **Codex** | ✅ | ✅ **Probe-verified** (v0.144.3, 2026-07-13) | `deny` honored; `allow`/`ask` parsed-but-unsupported; config nests under top-level `hooks`. |
| **Antigravity (`agy`)** | ✅ | ✅ **Verified live** (v1.1.2, 2026-07-13) | `deny`/`ask`/`force_ask` work; fails closed. **Supersedes Gemini.** |
| **Gemini** | (exit codes) | ⚪ **Deprecated** — do not test | Gemini CLI retired 2026-06-18 → replaced by `agy` (above). Target kept as an enterprise remnant; dropped from the active matrix. |
| **Qwen** | ✅ | ◻️ **Assumed** (Claude-Code mirror) | Declares Claude's `permissionDecision` shape; not independently exercised. |
| **Droid** | ✅ | ◻️ **Assumed** (Claude-Code mirror) | Same shape; shell tool is `Execute`, not `Bash`. |
| **Cursor** | ✅ | ✅ **Verified live → DENY harness** (`cursor-agent` v2026.07.16) | `deny` works (blocks + shows our message); `allow` is ignored (a known cursor bug). Decision: emit `deny` for gated commands so safe-chains is protective (like Codex); keep `allow` for safe (inert until cursor honors it). Revisit if the bug is fixed. See §Cursor. |
| **Copilot** | ✅ | ✅ **Verified live** (v1.0.71) | Hook FIRES (per-repo `.github/hooks/*.json`); allow AND deny BOTH honored (allow auto-runs + suppresses copilot's own prompt; deny blocks + shows our reason). Fixed two bugs: user path `~/.github/hooks` → **`~/.copilot/hooks`**, and the stale "deny-only" note. See §Copilot. |
| **opencode** | ❌ | 🚫 **Not integrated — `--opencode-config` DROPPED** (2026-07) | No usable hook (plugin hook broken, [#7006](https://github.com/anomalyco/opencode/issues/7006)); a static glob allowlist can't express per-arg safety. The flag/stub/renderer were removed; the target stays for detection only. **Watch #7006** to revisit. See §opencode. |

**Remaining live-verification work:** only Qwen/Droid (both "assumed" Claude-Code mirrors) — optionally
exercise to upgrade to "verified". Copilot is now verified live (allow+deny both honored — see §Copilot);
Cursor is done (a DENY harness, `allow` ignored on the CLI); Gemini is closed out via `agy`;
Claude/Codex/agy done; opencode not integrable.

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
| Cursor  | (settings `version: 1`) | flat object                            | `permission`        | **deny** honored; `allow`/`ask` IGNORED (allowlist wins — cursor bug) | — |
| Copilot | (no matcher; self-filter on `toolName`) | flat object, `toolArgs` is a nested JSON string | `permissionDecision` | **allow AND deny honored** (v1.0.71; was thought deny-only) | `timeoutSec` |

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

### Claude Code — VERIFIED LIVE, 2026-07 (in a running session)

Previously verified only from the Claude Code hooks docs; now confirmed end-to-end by running the
installed hook inside a live Claude Code session (this is the harness safe-chains dogfoods on):
- **Allow envelope works.** An allowlisted command (`cargo test`, `grep`, …) returns
  `{hookSpecificOutput: {permissionDecision: "allow", permissionDecisionReason}}` and is **auto-approved
  silently** — no prompt. Observed hundreds of times.
- **Abstain + context works.** A non-allowlisted command returns `additionalContext` with NO
  `permissionDecision`; Claude injects the "not auto-approved … This is not a block" text and the
  command proceeds through the **normal permission flow** (the user's own allowlist / a prompt) —
  exactly the abstain contract in `src/targets/claude.rs` (allow → `permissionDecision:"allow"`, else
  `additionalContext` only). `permissionDecision: "deny"`/`"ask"` are supported by the shape but
  safe-chains never emits them (allowlist-only, human keeps the review).
- **Capability summary:** grant ✅ (allow) · deny (unused) · human-review-on-silence ✅ (abstain →
  Claude's own prompt) · additionalContext ✅. Matches `src/targets/claude.rs`.

### Cursor CLI (`cursor-agent`) — VERIFIED LIVE, v2026.07.16, 2026-07 (with a finding)

Driven end-to-end via TUI automation, with a logging wrapper around `safe-chains hook cursor` proving
each invocation. Config: `~/.cursor/hooks.json` → `hooks.beforeShellExecution[].command`.
- **Hook fires; envelope matches our parser.** cursor-agent sends the documented
  `beforeShellExecution` payload — top-level `command`, `workspace_roots`, `cursor_version`, and
  (observed) an EMPTY `cwd` (our parser already falls back to `workspace_roots[0]` for the root).
- **`deny` WORKS.** A forced `{"permission":"deny", "user_message":...}` blocked even `echo`:
  *"The command did not run. Cursor blocked it with this hook message: <our message>."* So cursor
  honors `deny` and surfaces our text.
- **`allow` is IGNORED (the finding).** With `{"permission":"allow"}` — both from real safe-chains
  (`cat README.md`, a worktree read we allow) AND a forced always-allow wrapper — cursor STILL prompts
  *"Run this command? Not in allowlist: cat"*. Its own per-user command allowlist runs regardless of a
  hook `allow`. (Cursor's docs list `allow` as valid but don't say it bypasses the allowlist; live, it
  does not.) The in-app tip even says "/run-everything to skip all approvals" — hooks don't grant.
- **DECISION (2026-07): Cursor is a DENY harness** (`gated_policy() = Deny`). Since `allow` is the only
  broken lever and `deny` works, `src/targets/cursor.rs` now emits `permission:"deny"` (with our reason
  in `user_message`/`agent_message`) for a gated command, and keeps `permission:"allow"` for a safe one
  (inert until cursor honors it). The forum confirms the ignored-`allow` behaviour is a bug
  (forum.cursor.com/t/…/144244), so this is REVISITABLE: if cursor starts honoring `allow`, switch back
  to allow-for-safe + Defer (abstain) for gated. The Cursor IDE may already differ from the CLI (untested).
- **TRADE-OFF to remember:** like Codex, a Deny harness VETOES every not-allowlisted command — and on
  Cursor the veto is a hard block (no in-app "approve once"), so the escape valve is a
  `~/.config/safe-chains.toml` grant, not cursor's prompt. This is stricter than the prior abstain
  (which let cursor prompt); it is the price of protection while `allow` is broken.

### opencode — INERT (pattern generator is a stub), 2026-07

opencode has NO usable runtime hook: its `permission.ask` plugin hook is defined in the SDK but never
fires ([#7006](https://github.com/anomalyco/opencode/issues/7006)), so the only integration is a static
`opencode.json` `permission.bash` map (`"pattern" -> allow|ask|deny`, last match wins, `"*"` first).
`safe-chains --opencode-config` renders that map. Two findings from the 2026-07 check:
- **The generator is a NO-OP STUB.** `handlers::all_opencode_patterns()` builds an empty `Vec`,
  sorts/dedups nothing, returns it. So the rendered config is just `{"bash": {"*": "ask"}}` — it
  allowlists ZERO commands. safe-chains provides **no auto-approval** on opencode today. (No test
  asserts the pattern set is non-empty, so it slipped through — a test gap.)
- **Even filled in, the approach is FUNDAMENTALLY LOSSY.** opencode matches a command against glob
  prefixes; safe-chains classifies per-argument. `"git *": "allow"` would allow `git push` (gated) as
  well as `git status` (safe). The only SOUND patterns are commands safe for EVERY argument (the
  always-inert set: `echo`, `pwd`, `date`, `whoami`, …) — a small subset. Anything richer over-allows.
- **DECISION (2026-07): dropped `--opencode-config`.** It advertised an allowlist it never produced, so
  it was removed (flag, the `all_opencode_patterns()` stub, and the `render_opencode_json_in` renderer).
  `OpenCodeTarget` stays for DETECTION only: `install()` now returns a Skip that explains opencode has no
  usable hook yet (see the reason string). REVISIT when opencode ships a real runtime hook (#7006) — then
  a proper per-command integration (like the other targets) becomes possible; a static glob allowlist
  never can. **Watch [opencode #7006](https://github.com/anomalyco/opencode/issues/7006)** and opencode's
  permissions/plugin docs for that change.

### GitHub Copilot CLI — VERIFIED LIVE, v1.0.71, 2026-07

Driven end-to-end via TUI with a logging wrapper around `safe-chains hook copilot`, using a per-repo
`.github/hooks/safe-chains.json` (`{version, hooks.preToolUse[]}`, field `bash` = the script, no matcher).
- **Hook FIRES.** Despite the reports about `hooks.json` not firing ([#2540](https://github.com/github/copilot-cli/issues/2540)),
  the per-repo `.github/hooks/*.json` config fired on every bash tool call. The envelope matches our
  parser: `{"toolName":"bash","toolArgs":"{…command…}","cwd":…}` with `toolArgs` a JSON-encoded STRING
  (double-decode), and a flat response (no `hookSpecificOutput`).
- **BOTH `allow` and `deny` are honored** (the "deny-only" belief was wrong): a forced `deny` blocked
  `echo` — *"Denied by preToolUse hook: BLOCKED BY SAFE-CHAINS TEST"* — and a forced `allow` auto-ran
  `cat /etc/hosts` with NO prompt, where our abstaining hook had made copilot show its own directory-access
  prompt. So `allow` is a real grant and even suppresses copilot's own prompts.
- **Two BUGS fixed in `src/targets/copilot.rs`:** (1) the user-global install path was `~/.github/hooks/`,
  which does not exist — copilot's config root is **`~/.copilot/`**. The live drive used the per-repo
  `.github/hooks/` config; the user-global path our installer writes, **`~/.copilot/hooks/`** (or
  `$COPILOT_HOME/hooks/`), is confirmed by upstream's hooks-configuration docs, which state both the
  per-repo `.github/hooks/*.json` and `~/.copilot/hooks/*.json` are loaded and "all hook entries from all
  sources are run." (2) the `render_response` "only deny is processed" comment was stale.
- **Model:** Copilot is a Defer/grant harness like Claude — safe → `allow` (a real grant), gated →
  ABSTAIN (empty) → copilot's own per-command prompt (it has interactive review, so we don't hard-deny).
  Current `targets/copilot.rs` already does exactly this. (`additionalContext` on gated is still not
  wired — copilot may not pass it, [#2585](https://github.com/github/copilot-cli/issues/2585) — untested.)

## Model-visible context injection (`additionalContext`)

This is what powers the chain-explainer feedback (`cst::explain`). It injects
text into the model's context **without** a permission decision, so the normal
flow is untouched.

| Target  | `additionalContext` support | What we do |
|---------|-----------------------------|------------|
| Claude  | Yes (verified LIVE this session + docs) | emit on the mixed-chain case |
| Qwen    | Assumed (declares Claude-Code-mirror)   | emit on the mixed-chain case |
| Droid   | Assumed (declares Claude-Code-mirror)   | emit on the mixed-chain case |
| Codex   | Unverified                  | default abstain (emit nothing) |
| Cursor  | Unverified / different shape | default abstain |
| Gemini  | Deprecated — retired, superseded by `agy` (decision-only contract, no context field) | n/a |
| Copilot | Decision contract verified live (allow+deny); `additionalContext` likely not passed ([#2585](https://github.com/github/copilot-cli/issues/2585)) | default abstain |

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
