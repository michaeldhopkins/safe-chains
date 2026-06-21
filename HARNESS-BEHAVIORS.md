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

safe-chains is **allowlist-only and abstaining**. On a safe command it emits an
allow envelope; on anything else it emits *nothing* and exits 0, so the
harness's normal permission flow (and the user's own allowlist) still applies.
We never actively deny.

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
| Codex   | `Bash` (self-filter)    | `{hookSpecificOutput: {...}}`          | `permissionDecision`| allow                 | — |
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
