# Security

safe-chains is an allowlist-only command checker. It auto-approves bash commands that it can verify as safe. Any command not explicitly recognized is left for the human to approve.

## What it prevents

Auto-approval of destructive, write, or state-changing commands. An agentic tool cannot use safe-chains to bypass permission prompts for `rm`, `git push`, `sed -i`, `curl -X POST`, or any command/flag combination not in the allowlist.

## Security properties

**Allowlist-only.** Unrecognized commands are never approved.

**Per-segment validation.** Commands with shell operators (`&&`, `|`, `;`, `&`) are split into segments. Each segment must independently pass validation. One safe segment cannot cause another to be approved.

**Redirection handling.** Output redirection (`>`, `>>`) to `/dev/null` is `inert`. Output redirection to any other file is allowed at `safe-write` level. Input redirection (`<`), here-strings (`<<<`), and here-documents (`<<`, `<<-`) are allowed.

**Substitution validation.** Command substitutions (`$(...)`) are extracted and recursively validated. `echo $(git log)` is approved because `git log` is safe. `echo $(rm -rf /)` is not. `bash -c` and `sh -c` recursively validate their arguments.

**No shell evaluation.** `eval`, `exec`, `source`, and `.` (dot-source) are never approved when they would execute arbitrary commands.

**Settings guardrails.** When matching commands against your Claude Code settings patterns, segments containing `>`, `<`, backticks, or `$()` are never approved via settings, even if a pattern matches. This prevents `Bash(./script *)` from approving `./script > /etc/passwd`.

## What it does not prevent

- **Information disclosure.** Read-only commands can read sensitive files (`cat ~/.ssh/id_rsa`). Sensitive contents would be read by the model provider. We recommend pairing safe-chains with a hook to block reading `~/.ssh`, `../credentials` and similar directories.
- **Unrecognized commands.** Commands safe-chains doesn't handle are passed through to the normal permission flow, not blocked.
- **Broad user-approved patterns.** If you add patterns like `Bash(bash *)` to your Claude Code settings, safe-chains will match them per-segment without recursive validation, matching Claude Code's own behavior. See [Cleaning up approved commands](configuration.md#cleaning-up-approved-commands).

## What safe-chains is not

- **Not a sandbox.** It does not restrict what commands can do once they run. It only decides whether to auto-approve the permission prompt.
- **Not a firewall.** While unsafe commands are not approved, it does not filter at the network layer or file system operations layer.
- **Not an inspection tool.** It evaluates based on path, args and flags, not binary signatures.

## Testing approach

Every command handler is covered by multiple layers of automated testing:

- **Unknown flag rejection.** Every command is automatically tested to verify it rejects unknown flags and subcommands.
- **Property verification.** Systematic tests verify that `help_eligible` declarations match actual behavior, that `bare=false` policies reject bare invocations, that guarded subcommands require their guard flags, and that nested subcommands reject bare parent invocations.
- **Per-handler tests.** Each handler includes explicit safe/denied test cases covering expected approvals and rejections.
- **Registry completeness.** A test verifies that every command in the handled set has a corresponding entry in the test registry, and vice versa.

## Reporting vulnerabilities

{{#include includes/cta-vulnerability.md}}

## Contributing

{{#include includes/cta-new-command.md}}
