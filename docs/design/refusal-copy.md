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
That is not a rating of the command. safe-chains approves commands it has researched
and has no opinion about the rest. The normal approval prompt follows.
If `warnings` is a real command that should be approved, please open an issue:
https://github.com/michaeldhopkins/safe-chains/issues
```

### 2. Unknown command, we emit deny and the harness honours it (Codex, Cursor, Grok, agy)

```
safe-chains did not approve this, and the command did not run.
safe-chains has no entry for the command `warnings`. That is not a rating of the
command. safe-chains approves commands it has researched and has no opinion about the
rest.
If `warnings` is a real command that should be approved, please open an issue:
https://github.com/michaeldhopkins/safe-chains/issues
```

### 3. Unknown command, no harness detected (direct CLI, unrecognised target)

Vague about what follows, exact about what was found. Note the wording is "gives no answer" rather
than "says nothing": on some tools a non-approval is enforced as a block, so claiming safe-chains
stays out of it would be false there.

```
safe-chains has no entry for the command `warnings`, so it did not approve it.
That is not a rating of the command. safe-chains approves commands it has researched.
For anything else it gives no answer, and the tool that ran safe-chains decides what
to do by its own default.
If `warnings` is a real command that should be approved, please open an issue:
https://github.com/michaeldhopkins/safe-chains/issues
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

The command is known, so do not say "no entry". Name the path and the reason. Name the remedy that
WORKS: measured, a path grant does not open a credential path, while `--level local-admin` and
`--level yolo` both do. Offering the grant here would be the same defect as telling someone to grant
a path that is not a path.

```
safe-chains did not auto-approve this. It reads `~/.ssh/id_rsa`, which holds
credentials. If that was not what you meant to do, stop and check the command.
Adding this path to the granted paths in ~/.config/safe-chains.toml will not change
it. Credential paths are held back from path grants on purpose, so that widening a
directory cannot widen the keys inside it.
To allow reads like this, raise the level in ~/.config/safe-chains.toml to
local-admin or yolo.
```

### 6. A path built by an interpolation

Many interpolations ARE computed, so do not imply none are. Say which kinds work, and name the one
in this command that did not.

```
safe-chains did not auto-approve this. The path `./out/$(id)` depends on the output of
`id`, and safe-chains cannot tell where that points.
It can work this out for commands whose output it knows. `$(pwd)` is the working
directory. `$(find ./src -name x)` is a path under ./src. `$(seq 1 4)` is a number.
`id` is not one of those, so the path could be anywhere.
Two things help. Use a command safe-chains knows, or put literal text next to the
interpolation in the same path component: `out/dx_$i.txt` is approved where `out/$i`
is not, because the first is a filename whatever `$i` holds and the second could be
`..`.
```

### 7. Code run from a temporary directory

The read/write versus run distinction is the whole point, so state it as two facts rather than one
sentence with "even though" in the middle. Do NOT mention the session scratchpad unless a session id
was actually supplied. Claiming a scratchpad is recognised when none was reported sends the reader
looking for something that is not there.

```
safe-chains did not auto-approve this. It runs code from `/tmp/build.sh`.
Reading and writing files in a temporary directory is approved. Running code from one
is not, because a temporary directory is where a downloaded file lands, and safe-chains
cannot tell a script you wrote from one that arrived.
If this is a directory you trust, grant it in ~/.config/safe-chains.toml.
```

### 8. A path that is not in an approved place

"Outside the working directory" is WRONG and was measured so: at `developer`, `../peer/README.md`,
`/tmp/x`, `/usr/share/man/man1/ls.1` and `~/.cargo/registry/...` are all outside the working
directory and all approved, while `~/notes.txt`, `/etc/hosts`, `~/Documents/x.txt` and
`/usr/local/bin/tool` are refused. Name the places that ARE approved, and say the path is not one of
them. The list depends on the level, so it must be generated from the level in force rather than
hardcoded.

```
safe-chains did not auto-approve this. It reads `~/notes.txt`.
At the current level, reads are approved in the working directory, in projects next to
it, in temporary directories, and in installed package files. `~/notes.txt` is not in
any of those.
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

## Open UX problem this review exposed: two levers, one discoverable

Writing example 5 turned up an inconsistency worth fixing in the LOGIC, not just the copy.

Measured, at the default level:

```
--level developer      deny     cat ~/.ssh/id_rsa
--level local-admin    APPROVE
--level network-admin  deny
--level yolo           APPROVE
```

So a credential read IS reachable. Not through the lever a user would reach for first. A path grant
in `~/.config/safe-chains.toml` does not open it, on purpose, so that widening a directory cannot
widen the keys inside it. Raising the LEVEL does open it.

That is defensible and undiscoverable. Someone who wants to read their own `~/.ssh/config` grants the
path, sees no change, and has nothing telling them the other lever exists.

`local-admin` allowing it while `network-admin` refuses is also coherent but surprising. They are
siblings, not a ladder: `local-admin` flexes local machine access, `network-admin` flexes remote
egress, and a credential read is local. Nothing in the output says so.

Three things to fix, in order of how much they buy:

1. When a path is refused and a grant would NOT change it, say which lever would. Example 5 does
   this now. It is the cheapest fix and covers the common case.
2. When a path is refused and a grant WOULD change it, say so specifically. Examples 7 and 8 do
   this. The two messages must not be interchangeable, or the advice is noise.
3. Generate the "approved places" list from the level in force rather than hardcoding it. Example 8
   names four kinds of place, and all four are level-dependent. A hardcoded list is wrong the moment
   someone changes level, and wrong in the direction that teaches distrust.

A guard belongs with 1 and 2: for a refused path, the message must offer a grant if and only if a
grant actually changes the verdict. That is checkable by running the verdict twice, once with the
grant applied, and it fails the day the two drift.

## Sites to change

- `src/main.rs:300`. Currently "safe-chains **blocked** this: it is not on the allowlist and this
  harness has no …". Correct for a deny-harness; still needs the name and the not-a-judgement line.
- The `--explain` / no-approval path. Currently "this command is not on the allowlist, so it is not
  auto-approved".
- `ReachReason::message`. Already follows this spirit (it names the path and gives a remedy that
  exists); it is the model for the rest, not a site needing change.

## Partial-implementation risks

**Three producers, not one.** The gated reason is built once in `main.rs` and handed to
`render_deny`/`render_ask`, which every target wraps in its own envelope. That part is already
centralised and is the easy half. But the abstain path produces its text through `ReachReason`, and
`--explain` produces its own. Fixing one leaves the others, which is how "not on the allowlist"
survives in some outputs and not others today. Any implementation should route all three through one
builder, or the next reviewer finds the same inconsistency in a different place.

**The emission has to reach the builder.** Rule 1 says copy follows the (capability, emission) pair.
`main.rs` knows it, because it picks deny vs ask vs abstain from `gated_policy()`. The nudge path
does not necessarily know it. If the builder cannot see the emission it will default to one wording,
which is exactly the "blocked on a harness that only prompts" error.

**Target tests hardcode the copy.** `codex.rs`, `grok.rs` and `cursor.rs` each assert on literal
strings like `"blocked: not on the allowlist"`. Those keep passing while the real copy changes,
which is worse than no test: they report green on the layer being changed. They should assert
envelope SHAPE (field names, decision values, exit codes) and leave wording to the copy guards.

**The avoid-list guard must cover every producer.** A string check that only scans `main.rs` will
pass while `ReachReason` still says "blocked". Enumerate the producers, not the files you remember.

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
