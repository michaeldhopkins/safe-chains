# Refusal copy: what we say when we don't auto-approve

## Why this needs a spec

The reader is usually an AGENT meeting safe-chains for the first time, mid-hook, with no idea what
it is. The current line is:

> safe-chains: this command is not on the allowlist, so it is not auto-approved

Two problems, and the second is the dangerous one.

**"Allowlist" is jargon.** It carries no meaning to a reader who has never seen this tool.

**It reads as a verdict.** "Not on the allowlist" sounds like *the command was assessed and
rejected*. The natural response to that is to look for a spelling that passes. That is the one
behaviour we least want, and AGENTS.md already forbids contributors from doing it. The true
statement is nearly the opposite: safe-chains only ever GRANTS approvals for commands it has
researched. Silence about everything else is the default state, not a judgement.

A worked example of the cost. `RUSTDOCFLAGS=-D warnings cargo doc …` was refused because, unquoted,
the shell takes `warnings` as the COMMAND NAME. The message never said the word `warnings`, so the
refusal looked arbitrary. Naming it would have turned a friction moment into an instant bug report.
The bug was real and otherwise silent, since `bash: warnings: command not found` matches neither
`^error` nor `^warning` and would have been swallowed by the user's own grep.

## Rule 1. The copy follows the EMISSION, not the harness

The tempting shortcut is one message per harness. That is wrong: what happens next depends on what
we emit for THIS command on THIS harness. A deny-harness that we ABSTAIN on produces an ordinary
prompt, not a block.

So the copy is selected by the (capability, emission) pair that already drives the decision:

| what we emit | what actually happens | wording |
|---|---|---|
| `deny` (harness honors deny) | the command does not run | **blocked**. Accurate, so say it plainly |
| abstain, harness prompts | the harness's normal approval flow runs | **goes to a human** |
| `allow` | it runs | (no message) |
| unknown / no harness (CLI, `--explain`) | we cannot know | see Rule 4 |

Deriving the copy from the emission means it cannot drift from behaviour: if the emission changes
for a harness (as Cursor's did when `allow` turned out to be ignored), the copy follows without a
separate edit. Any implementation that hardcodes "blocked" per target name reintroduces exactly the
drift `HARNESS-BEHAVIORS.md` exists to prevent.

Harness classes as of 2026-08 (see `HARNESS-BEHAVIORS.md`, which is authoritative):

- **allow honored, abstain prompts**. Claude, Copilot, Qwen*, Droid*. Wording: "goes to a human"
- **deny harnesses** (`allow` only declines to deny). Codex, Cursor, Grok, agy. Wording: "blocked"
  WHEN we emit deny, still "goes to a human" when we abstain
- **no harness**. Direct CLI, `--explain`, unrecognised target. See Rule 4

## Rule 2. Always name what was resolved

The single most useful fact is the command name we actually resolved, and it is currently absent.

> safe-chains has no entry for the command `warnings`

Not "this command". The name. When the refusal is a parse surprise, that word IS the explanation.

## Rule 3. Neutral vocabulary

State what happened and what is next. Do not characterise the command.

Avoid: *not allowed, rejected, forbidden, dangerous, unsafe, suspicious, violation, denied by
policy*. Also avoid *allowlist* as a bare noun in agent-facing copy.

Prefer: *has no entry for*, *did not auto-approve*, *goes to a human for approval*, *did not run*.

Say once, plainly, that this is not an appraisal:

> That is not a rating of the command. safe-chains approves only commands it has researched, and
> says nothing about the rest.

And close the evasion path explicitly, because an agent reading a refusal will otherwise try
spellings until one passes:

> Rewriting the command to get it approved is not the fix.

## Rule 4. The fallback is vague about CONSEQUENCE, exact about CAUSE

When the harness is unknown we do not know whether the command will be blocked, prompted, or run
anyway. Do not guess. Be specific about what we found and silent about what follows:

> safe-chains has no entry for the command `warnings`, so it did not auto-approve it. What happens
> next depends on the tool that called safe-chains.

The failure mode to avoid is a confident "this was blocked" that turns out false. A reader who
catches us being wrong about the consequence has no reason to trust us about the cause.

## Rule 5. The parse-surprise variant

When the resolved command name is an unknown bare word AND the line carries an env-assignment prefix
(`VAR=value cmd`), a missing quote is overwhelmingly the explanation. That shape is detectable, and
one extra sentence earns its place:

> The command name here is `warnings`. It comes after the `RUSTDOCFLAGS=-D` assignment, so the shell
> reads it as the program to run. If you meant `-D warnings` as one value, it needs quotes.

Do not emit this speculatively. Only when both conditions hold. A hint that is wrong half the time
is worse than none, because it teaches the reader to skip the explanation.

## Rule 6. Plain words, and no em dashes

The copy is read by people under time pressure and by models that will imitate it. Write it the way
a careful colleague talks.

- No em dashes. Use a full stop, or a comma, or two sentences.
- No semicolons in agent-facing copy. Split the sentence.
- Short sentences. One idea each.
- No filler: *simply, just, note that, it is worth noting, importantly, essentially, in order to*.
- No hedging that adds nothing: *may potentially, appears to be, seems like*.
- British or American spelling, pick one and match the file. Prefer *rating* over *judgement* so the
  question does not arise.
- Say "safe-chains" in lower case, as the binary is named.

Every example below obeys these. They are the deliverable of this spec: implement the copy to match
them, not to paraphrase them.

## Example outputs

### 1. Unknown command, harness asks the human (Claude, Copilot, Qwen, Droid)

```
safe-chains has no entry for the command `warnings`, so it did not auto-approve this.
That is not a rating of the command. safe-chains approves only commands it has
researched, and says nothing about the rest. The normal approval prompt follows.
Rewording the command to get it approved is not the fix.
```

### 2. Unknown command, we emit deny and the harness honours it (Codex, Cursor, Grok, agy)

```
safe-chains did not approve this, and the command did not run.
safe-chains has no entry for the command `warnings`. That is not a rating of the
command. safe-chains approves only commands it has researched, and says nothing about
the rest. Rewording the command to get it approved is not the fix.
```

### 3. Unknown command, no harness detected (direct CLI, unrecognised target)

Vague about what follows, exact about what was found.

```
safe-chains has no entry for the command `warnings`, so it did not auto-approve this.
What happens next depends on the tool that ran safe-chains.
That is not a rating of the command. safe-chains approves only commands it has
researched, and says nothing about the rest.
```

### 4. Parse surprise: an env assignment swallowed the command name

Only when the resolved name is an unknown bare word AND an assignment prefix is present.

```
safe-chains has no entry for the command `warnings`, so it did not auto-approve this.
The command name here is `warnings`. It comes after the `RUSTDOCFLAGS=-D` assignment,
so the shell reads it as the program to run. If you meant `-D warnings` as one value,
it needs quotes: RUSTDOCFLAGS="-D warnings".
The normal approval prompt follows.
```

### 5. A known command reaching a credential store

The command is known, so do not say "no entry". Name the path and the reason.

```
safe-chains did not auto-approve this. It reads `~/.ssh/id_rsa`, which holds
credentials. If that was not what you meant to do, stop and check the command.
Granting this path in ~/.config/safe-chains.toml will not help. Credential paths are
never auto-approved.
```

### 6. A path built by an interpolation

Give the remedy that exists, and only if it exists.

```
safe-chains did not auto-approve this. The path `./out/$(id)` is built by an
interpolation, so what it names depends on a value that is not in the command. It
could be any path, which is why it cannot be approved in advance.
If the interpolated part cannot contain a `/`, putting literal text next to it in the
same path component is enough: `out/dx_$i.txt` is approved where `out/$i` is not,
because the first is a filename whatever `$i` holds and the second could be `..`.
```

### 7. Code run from a temporary directory

```
safe-chains did not auto-approve this. It runs code from `/tmp/build.sh`. Temporary
directories are where downloaded files land, so code there is treated as foreign even
though reading and writing temp files is fine.
If this is a directory you trust, grant it in ~/.config/safe-chains.toml. A scratchpad
that the harness reports for this session is recognised already and needs no grant.
```

### 8. A path outside the working directory

```
safe-chains did not auto-approve this. It reads `~/other-project/notes.md`, which is
outside the working directory `~/projects/app`.
If safe-chains is running from the wrong directory, restart it where you meant to be.
To allow this path from here, grant it in ~/.config/safe-chains.toml.
```

### 9. Part of a chain was not approved

The whole chain needs approval when any segment does. Say which one.

```
safe-chains approved 4 of the 5 commands here. This one is not approved:
  warnings cargo doc --workspace --no-deps
Splitting the unapproved command into its own call keeps the rest running without a
prompt.
```

### 10. What NOT to write

Each of these is a real failure mode, not a hypothetical.

```
safe-chains blocked this: it is not on the allowlist.
```
Two faults. "Allowlist" means nothing to a first-time reader, and "blocked" is false on a harness
that only prompts.

```
This command is not allowed and may be dangerous.
```
Rates the command, which safe-chains has no basis to do. It has no entry, that is all.

```
safe-chains could not verify this command was safe — you may want to try a different
approach.
```
An em dash, a hedge, and an invitation to go looking for a spelling that passes.

## Sites to change

- `src/main.rs:300`. Currently "safe-chains **blocked** this: it is not on the allowlist and this
  harness has no …". Correct for a deny-harness; still needs the name and the not-a-judgement line.
- The `--explain` / no-approval path. Currently "this command is not on the allowlist, so it is not
  auto-approved".
- `ReachReason::message`. Already follows this spirit (it names the path and gives a remedy that
  exists); it is the model for the rest, not a site needing change.

## Testing

The point of Rule 1 is that copy and behaviour cannot diverge, so the guard asserts the PAIRING:

1. For every target, for a command we abstain on, the message must not claim the command was
   blocked; for a command we emit `deny` on, it must not claim a human will be asked. Enumerate over
   `targets::registry()` so a new target is covered on the day it lands.
2. No agent-facing message contains a word from the Rule 3 avoid-list. A string-level guard, cheap
   and hard to argue with.
3. The resolved command name appears in the message. Red-demo by removing it.
4. The parse-surprise sentence appears for `VAR=value unknowncmd` and NOT for a plain unknown
   command.

Every one of these needs a red demo. Three of this session's findings were in this exact layer, and
each looked correct until the demo showed the message had not changed.
