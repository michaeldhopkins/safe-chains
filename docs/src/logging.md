# Logging

safe-chains can optionally log its decisions. These logs are local-only JSON dumps of commands, why they were approved or rejected, and the context from the harness. The log is created at `~/.local/state/safe-chains/log.jsonl`.

Sample log entry:

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
  "cwd": "/Users/you/projects/abc",
  "root": "/Users/you/projects/abc",
  "session_id": "c63f6e43-c07a-43e1-92ae-217729f06dbc",
  "triage": "unknown-command",
  "unknown_commands": ["mise"],
  "segments": [
    {
      "text": "mise exec --cd /w -- ruby -S bundle exec rspec 2>&1 | tail -4",
      "verdict": "denied",
      "culprit": "mise",
      "facets": null
    }
  ],
  "stateful": false
}
```

## Which segment, and why

Each segment of a chain gets its own verdict, and a denied one carries the reason it was refused.
That is the field to reach for first — it answers "why" without re-running anything:

```bash
jq -r '.segments[] | select(.verdict == "denied")
       | "\(.facets.refused_by.clause // "unknown command")  |  \(.text)"' \
  ~/.local/state/safe-chains/log.jsonl
```

```
locus.local = machine (allowed: <= adjacent)  |  sed -i '' '1257,$d' app/x.css
unknown command                               |  mise exec -- ruby -S bundle exec rspec
```

`facets` is null when no resolver claimed the command — an unrecognized tool has no profile to
report, and `triage` plus `unknown_commands` are the answer for that class instead.

## Turning it on

Add `--log` to the hook command:

```json
"hooks": {
  "PreToolUse": [
    {
      "matcher": "Bash",
      "hooks": [
        {
          "type": "command",
          "command": "safe-chains --log"
        }
      ]
    }
  ]
}
```

## Options

`--log`: by default, safe-chains logs denies/fail-to-approve only.
`--log-everything`: logs both denies and approvals.

## An entry is not a block

`"outcome": "denied"` means safe-chains did not *auto-approve* the command — it went to the normal
permission prompt. Most of what lands here was then approved by you and ran. The log records what
safe-chains decided, not what happened next.

## Reading it from inside a project

Reading the log is itself a command, and the log lives outside your project, so running `jq` against
it from a repo will prompt:

```
safe-chains did not auto-approve this: it reaches `~/.local/state/safe-chains/log.jsonl`,
outside the working directory.
```

That is the same rule that keeps an agent from reading the file, working as intended — it does not
know you are the one asking. Approve the prompt, or grant the path in `~/.config/safe-chains.toml`
if you read the log often.

## Housekeeping

The log rotates at 16 MB, keeping five older generations: `log.jsonl` becomes `log.jsonl.1`, the
previous `.1` becomes `.2`, and so on, with `.5` dropped. That is the same shape macOS uses in
`/etc/newsyslog.conf` (1 MB, count 5) and logrotate uses in its own example (`rotate 5`), sized up
because a JSON entry runs ~750 bytes against a syslog line's ~100.

Rotation is by size rather than on a schedule because safe-chains is a short-lived hook process
rather than a daemon — there is no cron job to run a weekly rotation, so the size is checked when
the file is opened.

To read across generations:

```bash
cat ~/.local/state/safe-chains/log.jsonl.5 ~/.local/state/safe-chains/log.jsonl.[4321] \
    ~/.local/state/safe-chains/log.jsonl 2>/dev/null | jq -s 'length'
```

Logging never affects classification. If the log can't be written — missing directory, full disk,
read-only filesystem — the command is classified exactly as it would be with logging off and nothing
is written.
