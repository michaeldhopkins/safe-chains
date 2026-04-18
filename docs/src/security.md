# Security

safe-chains is an allowlist-only command checker. It auto-approves bash commands that are verifiably read-only. Any command not explicitly recognized as safe is left for the human to approve.

## Security properties

**Allowlist-only.** Unrecognized commands are never approved.

**Per-segment validation.** Commands with shell operators (`&&`, `|`, `;`, `&`) are split into segments. Each segment must independently pass validation.

**Redirection handling.** Output redirection (`>`, `>>`) to `/dev/null` is `inert`. Output redirection to any other file is allowed at `safe-write` level. Input redirection (`<`), here-strings (`<<<`), and here-documents (`<<`, `<<-`) are allowed. When matching against Claude Code settings patterns, redirections are treated more conservatively — segments containing `>`, `<`, backticks, or `$()` are never approved via settings.

**Substitution validation.** Command substitutions (`$(...)`) are extracted and their contents are recursively validated.

**No shell evaluation.** `eval`, `exec`, `source`, and `.` (dot-source) are never approved when they would execute arbitrary commands.

## What safe-chains is not

- **Not a sandbox.** It does not restrict what commands can do once they run.
- **Not a firewall.** While unsafe commands are not approved, it does not filter at the network layer or file system operations layer.
- **Not an inspection tool.** It evaluates based on path, args and flags, not binary signatures.

## Reporting vulnerabilities

{{#include includes/cta-vulnerability.md}}

## Contributing

{{#include includes/cta-new-command.md}}
