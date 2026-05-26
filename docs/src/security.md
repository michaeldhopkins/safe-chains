# Security

safe-chains is an allowlist-only command checker. It auto-approves bash commands that it can verify as safe. Any command not explicitly recognized is left for the human to approve.

## What it prevents

Auto-approval of destructive, write, or state-changing commands. An agentic tool cannot use safe-chains to bypass permission prompts for `rm`, `git push`, `sed -i`, `curl -X POST`, or any command/flag combination not in the allowlist.

## Security properties

- Allowlist-only: unrecognized commands are never approved.

- Per-segment validation: commands with shell operators (`&&`, `|`, `;`, `&`) are split into segments that are independently evaluated. All segments must return safe to approve the command.

Settings guardrails: when matching commands against your Claude Code settings patterns, segments containing `>`, `<`, backticks, or `$()` are never approved via settings, even if a pattern matches. This prevents `Bash(./script *)` from approving `./script > /etc/passwd`.

## What it does not prevent

- Information disclosure: read-only commands can read sensitive files (`cat ~/.ssh/id_rsa`). Sensitive contents would be read by the model provider. We recommend pairing safe-chains with a hook to block reading `~/.ssh`, `../credentials` and similar directories.
- Unrecognized commands: commands safe-chains doesn't handle are passed through to the normal permission flow for your harness.
- Chaining with broad approvals: if you add patterns like `Bash(bash *)` to your Claude Code settings, safe-chains will match them per-segment without recursive validation, matching Claude Code's own behavior. See [Cleaning up approved commands](configuration.md#cleaning-up-approved-commands).

## Testing approach

Every command handler is covered by multiple layers of automated testing. Each handler includes explicit safe/denied test cases covering expected approvals and rejections. Every command has a spec suite. Every *type* of data definition in safe-chains has a test suite. 

## Reporting vulnerabilities

{{#include includes/cta-vulnerability.md}}

## Contributing

{{#include includes/cta-new-command.md}}
