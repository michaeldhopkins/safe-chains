# Security

safe-chains is an allowlist-only command checker. It auto-approves bash commands that it can verify as safe. Any command not explicitly recognized is left for the human to approve.

## What it prevents

Auto-approval of destructive, write, or state-changing commands. An agentic tool cannot use safe-chains to bypass permission prompts for `rm`, `git push`, `sed -i`, `curl -X POST`, or any command/flag combination not in the allowlist.

Auto-approval of reads and writes outside your project. File commands are checked by location, so `cat ~/.ssh/id_rsa`, `cp secret /etc/x`, and writes to system paths are not approved. See [Files by location](how-it-works.md#files-by-location).

Auto-approval of writes to safe-chains' own config. See [Trusted directories](how-it-works.md#trusted-directories).

## Security properties

- Allowlist-only: unrecognized commands are never approved.

- Per-segment validation: commands with shell operators (`&&`, `|`, `;`, `&`) are split into segments that are independently evaluated. All segments must return safe to approve the command.

Settings guardrails: when matching commands against your Claude Code settings patterns, segments containing `>`, `<`, backticks, or `$()` are never approved via settings, even if a pattern matches. This prevents `Bash(./script *)` from approving `./script > /etc/passwd`.

## Trusted configuration

A project's `.safe-chains.toml` must be trusted before safe-chains honors it. See [Custom Commands](custom-commands.md#project-files-must-be-trusted).

## What it does not prevent

- Information disclosure inside your project: files in your project (and any [trusted directories](how-it-works.md#trusted-directories) you grant) are readable, so `cat .env` sends their contents to the model provider. safe-chains gates by location, not by content; it won't stop a read inside a directory you've allowed.
- Filesystem confinement: safe-chains is NOT a sandbox. If you manually approve a dangerous command, safe-chains will not stop it. If you replace a safe binary with an evil binary, safe-chains will let it run. It only reads command strings, statically. For real confinement, pair it with an OS sandbox or the harness's own file controls. safe-chains' job is to auto-approve the safe commands so the prompts you DO get are meaningful.
- Unrecognized commands: commands safe-chains doesn't handle are passed through to the normal permission flow for your harness.
- Chaining with broad approvals: if you add patterns like `Bash(bash *)` to your Claude Code settings, safe-chains will match them per-segment without recursive validation, matching Claude Code's own behavior. See [Cleaning up approved commands](configuration.md#cleaning-up-approved-commands).

## Best practices

**Don't run an agent from your home directory.** safe-chains treats your working directory as the project, and files under it are read/write. From `~`, a relative path like `cat Documents/finances.csv` is indistinguishable from a project file, so files throughout your home directory are exposed. Run agents from a project directory instead.

**Grant narrowly.** A [trusted directory](how-it-works.md#trusted-directories) grant is a deliberate broad allow, the same broad access you get by running from a directory. Grant the specific directories you work in (`~/projects`), not all of `~`.

**Deny the folders you never want touched.** On Claude Code, safe-chains only ever *approves*. It never denies, so a command it doesn't recognize just falls through to Claude's normal prompt. For a hard block on paths you never want touched, pair it with a second hook that denies them (Claude Code blocks the command if any hook denies it). A blunt backstop at `~/.claude/hooks/deny-paths.sh`:

```bash
#!/bin/bash
cmd=$(jq -r '.tool_input.command // ""')
case "$cmd" in
  *.ssh/*|*.aws/*|*.gnupg/*|*/.env|*/secrets/*)
    jq -n '{hookSpecificOutput:{hookEventName:"PreToolUse",permissionDecision:"deny",permissionDecisionReason:"protected path"}}' ;;
esac
```

Register it alongside safe-chains in `~/.claude/settings.json`:

```json
"hooks": { "PreToolUse": [ { "matcher": "Bash", "hooks": [
  { "type": "command", "command": "bash ~/.claude/hooks/deny-paths.sh" },
  { "type": "command", "command": "safe-chains" }
] } ] }
```

It's a string match, not a parser: a safety net for the obvious cases while safe-chains does the rigorous allow-listing.

## Testing approach

Every command handler is covered by multiple layers of automated testing. Each handler includes explicit safe/denied test cases covering expected approvals and rejections. Every command has a spec suite. Every *type* of data definition in safe-chains has a test suite. 

## Reporting vulnerabilities

{{#include includes/cta-vulnerability.md}}

## Contributing

{{#include includes/cta-new-command.md}}
