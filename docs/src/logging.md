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
      "culprit": "mise"
    }
  ],
  "stateful": false,
  "facets": null
}
```

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

## Housekeeping

The log rotates at a size cap: `log.jsonl` becomes `log.jsonl.1` and a fresh file starts. One
previous file is kept.

Logging never affects classification. If the log can't be written — missing directory, full disk,
read-only filesystem — the command is classified exactly as it would be with logging off and nothing
is written.
