# Security overview

safe-chains is an allowlist-only command checker. In a Bash tool use hook, auto-approves bash commands that are verifiably read-only — commands that inspect, query, or display information without modifying files, state, or external systems. Any command not explicitly recognized as safe is not necessarily harmful, but is left for the human to approve.

safe-chains is a response to the threat of *prompt fatigue*. Users of agentic tools are prompted for dozens of safe commands, especially when they start working in a new repo or on a new computer. These safe commands vary in structure and are nested or wrapped in logic blocks (if do, end, etc.) Managing permissions is tiring, making users more likely to auto-approve unsafe commands to make notifications go away, or to manually approve so many commands that they don't notice a dangerous permission prompt.

safe-chains is also a response to inadequate deny-based hooks in agentic tools. The denylists are inadequate and miss all the ways that CLI tools can be creatively called and chained together. The answer to this is an allowlist that's actually comprehensive and has an understanding of all kinds of flags, separator syntax, and parses nested bash to any depth.

**What it prevents:** 
1. Auto-approval of destructive, write, or state-changing commands. An agentic tool cannot use safe-chains to bypass permission prompts for `rm`, `git push`, `sed -i`, `curl -X POST`, or any command/flag combination not in the allowlist.

**What it does not prevent:**

- **Information disclosure.** Read-only commands can read sensitive files (`cat ~/.ssh/id_rsa` if the user approves the directory). While writing and sending tools are not allowed, sensitive contents would be read by the model provider. We recommend pairing safe-chains with a hook to block reading `~/.ssh`, `../credentials` and similar directories.
- **Commands it doesn't handle.** Unrecognized commands are not blocked. They are passed through to the normal permission flow, e.g., in Claude Code, to allow once or always allow.
- **Attacks through allowed patterns.** If you add broad patterns like `Bash(bash *)` to your Claude Code settings, safe-chains will match them per-segment without recursive validation, matching Claude Code's own behavior. We recommend removing most of your allowed bash commands when you start to use safe-chains as most should no longer be needed.

## Security properties

**Allowlist-only.** Unrecognized commands are never approved. The full list of recognized commands is in [COMMANDS.md](COMMANDS.md).

**Per-segment validation.** Commands with shell operators (`&&`, `|`, `;`, `&`) are split into segments. Each segment must independently pass validation. One safe segment cannot cause another to be approved.

**No file output.** Output redirection (`>`, `>>`) to any file is always rejected. The sole exception is `/dev/null`. Here-strings (`<<<`) are allowed since they provide input, not output.

**Substitution validation.** Command substitutions (`$(...)`) are extracted and their contents are recursively validated. `echo $(git log)` is approved because `git log` is safe. `echo $(rm -rf /)` is rejected because `rm` is not.

**No shell evaluation.** `eval`, `exec`, `source`, and `.` (dot-source) are never approved when they would execute arbitrary commands. `bash -c` and `sh -c` recursively validate their arguments.

**Settings guardrails.** When matching commands against your Claude Code settings patterns, segments containing `>`, `<`, backticks, or `$()` are rejected regardless of pattern matches. This prevents `Bash(./script *)` from approving `./script > /etc/passwd`.

## What safe-chains is not

- **Not a sandbox.** It does not restrict what commands can do once they run. It only decides whether to auto-approve the permission prompt.
- **Not a firewall.** It does not filter network access or file system operations.
- **Not an inspection tool.** It evaluates based on path, args and flags, not binary signatures.
- **Not a replacement for code review.** Since it speeds up development, it actually increases the need for code review. :)

## Testing approach

Every command handler is covered by multiple layers of automated testing:

- **Unknown flag rejection.** Every command is automatically tested to verify it rejects unknown flags and subcommands.
- **Property verification.** Systematic tests verify that `help_eligible` declarations match actual behavior, that `bare=false` policies reject bare invocations, that guarded subcommands require their guard flags, and that nested subcommands reject bare parent invocations.
- **Per-handler tests.** Each handler includes explicit safe/denied test cases covering expected approvals and rejections.
- **Registry completeness.** A test verifies that every command in the handled set has a corresponding entry in the test registry, and vice versa.

## Reporting vulnerabilities

If you find a command that safe-chains approves but shouldn't, please [open an issue](https://github.com/michaeldhopkins/safe-chains/issues/new). Include the exact command string and why it's unsafe.
