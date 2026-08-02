# Refusal copy: what we say when we don't auto-approve

## Why this needs a spec

The reader is usually an AGENT meeting safe-chains for the first time, mid-hook, with no idea what
it is. The current line is:

> safe-chains: this command is not on the allowlist, so it is not auto-approved

Two problems, and the second is the dangerous one.

**"Allowlist" is jargon.** It carries no meaning to a reader who has never seen this tool.

**It reads as a verdict.** "Not on the allowlist" sounds like *the command was assessed and
rejected*. The natural response to that is to look for a spelling that passes — which is the one
behaviour we least want, and which AGENTS.md already forbids contributors from doing. The true
statement is nearly the opposite: safe-chains only ever GRANTS approvals for commands it has
researched. Silence about everything else is the default state, not a judgement.

A worked example of the cost. `RUSTDOCFLAGS=-D warnings cargo doc …` was refused because, unquoted,
the shell takes `warnings` as the COMMAND NAME. The message never said the word `warnings`, so the
refusal looked arbitrary. Naming it would have turned a friction moment into an instant bug report —
and the bug was real and otherwise silent, since `bash: warnings: command not found` matches neither
`^error` nor `^warning` and would have been swallowed by the user's own grep.

## Rule 1 — the copy follows the EMISSION, not the harness

The tempting shortcut is one message per harness. That is wrong: what happens next depends on what
we emit for THIS command on THIS harness. A deny-harness that we ABSTAIN on produces an ordinary
prompt, not a block.

So the copy is selected by the (capability, emission) pair that already drives the decision:

| what we emit | what actually happens | wording |
|---|---|---|
| `deny` (harness honors deny) | the command does not run | **blocked** — accurate, say so plainly |
| abstain, harness prompts | the harness's normal approval flow runs | **goes to a human** |
| `allow` | it runs | (no message) |
| unknown / no harness (CLI, `--explain`) | we cannot know | see Rule 4 |

Deriving the copy from the emission means it cannot drift from behaviour: if the emission changes
for a harness (as Cursor's did when `allow` turned out to be ignored), the copy follows without a
separate edit. Any implementation that hardcodes "blocked" per target name reintroduces exactly the
drift `HARNESS-BEHAVIORS.md` exists to prevent.

Harness classes as of 2026-08 (see `HARNESS-BEHAVIORS.md`, which is authoritative):

- **allow honored, abstain prompts** — Claude, Copilot, Qwen*, Droid*  → "goes to a human"
- **deny harnesses** (`allow` only declines to deny) — Codex, Cursor, Grok, agy → "blocked" WHEN we
  emit deny; still "goes to a human" when we abstain
- **no harness** — direct CLI, `--explain`, unrecognised target → Rule 4

## Rule 2 — always name what was resolved

The single most useful fact is the command name we actually resolved, and it is currently absent.

> safe-chains has no entry for the command `warnings`

Not "this command". The name. When the refusal is a parse surprise, that word IS the explanation.

## Rule 3 — neutral vocabulary

State what happened and what is next. Do not characterise the command.

Avoid: *not allowed, rejected, forbidden, dangerous, unsafe, suspicious, violation, denied by policy*
— and avoid *allowlist* as a bare noun in agent-facing copy.

Prefer: *has no entry for*, *did not auto-approve*, *goes to a human for approval*, *did not run*.

Say once, plainly, that this is not an appraisal:

> That is not a judgement about the command — safe-chains only grants approvals for commands it has
> researched, and says nothing about the rest.

And close the evasion path explicitly, because an agent reading a refusal will otherwise try
spellings until one passes:

> Rewriting the command to get it approved is not the fix.

## Rule 4 — the fallback must be vague about CONSEQUENCE, not about CAUSE

When the harness is unknown we do not know whether the command will be blocked, prompted, or run
anyway. Do not guess. Be specific about what we found and silent about what follows:

> safe-chains has no entry for the command `warnings`, so it did not auto-approve it. What happens
> next depends on the tool that called safe-chains.

The failure mode to avoid is a confident "this was blocked" that turns out false — a reader who
catches us being wrong about the consequence has no reason to trust us about the cause.

## Rule 5 — the parse-surprise variant

When the resolved command name is an unknown bare word AND the line carries an env-assignment prefix
(`VAR=value cmd`), a missing quote is overwhelmingly the explanation. That shape is detectable, and
one extra sentence earns its place:

> The command name here is `warnings`, taken from after the `RUSTDOCFLAGS=-D` assignment. If you
> meant that as a single value it needs quoting.

Do not emit this speculatively — only when both conditions hold. A hint that is wrong half the time
is worse than none, because it teaches the reader to skip the explanation.

## Sites to change

- `src/main.rs:300` — currently "safe-chains **blocked** this: it is not on the allowlist and this
  harness has no …". Correct for a deny-harness; still needs the name and the not-a-judgement line.
- The `--explain` / no-approval path — "this command is not on the allowlist, so it is not
  auto-approved".
- `ReachReason::message` — already follows this spirit (it names the path and gives a remedy that
  exists); it is the model for the rest, not a site needing change.

## Testing

The point of Rule 1 is that copy and behaviour cannot diverge, so the guard asserts the PAIRING:

1. For every target, for a command we abstain on, the message must not claim the command was
   blocked; for a command we emit `deny` on, it must not claim a human will be asked. Enumerate over
   `targets::registry()` so a new target is covered on the day it lands.
2. No agent-facing message contains a word from the Rule 3 avoid-list. A string-level guard, cheap
   and hard to argue with.
3. The resolved command name appears in the message — red-demo by removing it.
4. The parse-surprise sentence appears for `VAR=value unknowncmd` and NOT for a plain unknown
   command.

Every one of these needs a red demo. Three of this session's findings were in this exact layer, and
each looked correct until the demo showed the message had not changed.
