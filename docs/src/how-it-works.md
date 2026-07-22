# How It Works

## Built-in rules

safe-chains knows {{#include includes/command-count.md}} commands. For each one it validates specific subcommands and flags, allowing `git log` but not `git push`, allowing `sed 's/foo/bar/'` but not `sed -i`.

## Files by location

When you run a bash command, in addition to checking the safety of the actual command, safe-chains checks the directory the command wants to affect. Generally, commands are approved when they operate within the _current working directory_ (from where you are running the agent, like `~/projects/abc`). It also approves some read and write locations outside of that, like `/tmp`.

If you run an agent from a high-level directory like `~/`, you give it a lot of power. This is the case whether or not you run safe-chains. Careful!

```bash
cat ./src/main.rs         # approved: inside your working directory
echo hi > ./out.txt       # approved: writing inside the project
grep -r TODO ./src        # approved
cat /tmp/scratch.txt      # approved: /tmp is scratch
cat /etc/hosts            # not approved: outside the project; you're prompted
cat ~/.ssh/id_rsa         # not approved: a credential
cp notes.txt /etc/x       # not approved: writing outside the project
```

safe-chains allows reaching into sibling directories of the current working directory. E.g., when working in `~/projects/webapp`, otherwise safe commands in `~/projects/mobileapp` would be auto-approved, except deleting a sibling's files. "Nephew" directories (e.g. `~/projects/mobileapp/android/config`) are also approved. This does not apply when you're working in children of user folders, root, etc.

## Trusted directories

If you always want to allow reading and writing in additional directories, add them to `~/.config/safe-chains.toml` with `read = true` and/or `write = true`. The binary will pick up these preferences. safe-chains has a special block to not auto-allow writes to `~/.config/safe-chains.toml`. This is because currently, most harnesses don't have a protected way for third party hooks to store preferences.

```toml
# Work across every project under ~/projects, not just the current one
[[grant]]
path = "~/projects/"
read = true
write = true

# A scripts directory the agent both runs and edits
[[grant]]
path = "~/.runner-scripts/"
read = true
write = true

# Read a toolchain's install dir, but never let the agent write to it
[[grant]]
path = "~/.local/share/mise/"
read = true
```

Dot files are generally not part of allowlisted target directories. You'll need to grant access to these directories explicitly. Credential stores like `.ssh`/`.aws` stay protected regardless.

### Read approvals from `~/.claude/settings.json`

If you use Claude Code, safe-chains also honors the file-**read** approvals already in your `~/.claude/settings.json`. A `permissions.allow` entry such as `Read(//Users/you/.local/share/mise/**)` or `Read(~/.gem/**)` becomes a read-only trusted directory — the same effect as a `[[grant]]` with `read = true`, so you don't have to declare a directory in two places. Only absolute (`//…`) and home (`~/…`) paths are honored; a bare "read anything" rule is not (grant that explicitly in `safe-chains.toml` if you really want it). `Edit(…)`/`Write(…)` rules are deliberately **not** turned into write grants — reads only — and the credential shields and dotfile rule above still apply, so a broad read rule can't reach `.ssh`. Only your user-level `~/.claude/settings.json` is read, never a project's `.claude/settings.json`.

## Parsing example

Take this command from the introduction:

```bash
find src -name "*.rs" -exec grep -l "TODO" {} \; | sort | while read f; do echo "=== $f ==="; grep -n "TODO" "$f"; done
```

Normally, you would be prompted to run this by your agent, and you would have to run some sort of auto- or permission-skipping mode to not be prompted, which could allow anything to be run.

Running [from a hook](#installation.md), safe-chains parses this and validates every leaf:

1. **Pipeline segment 1:** `find src -name "*.rs" -exec grep -l "TODO" {} \;`
   - `find` is allowed with positional predicates
   - `-exec` triggers delegation: the inner command `grep -l "TODO" {}` is extracted and validated separately
   - `grep -l` passes (`-l` is an allowed flag)
2. **Pipeline segment 2:** `sort` passes
3. **Pipeline segment 3:** `while read f; do ...; done` is a compound command, parsed recursively:
   - `read f` passes (shell builtin)
   - `echo "=== $f ==="` passes
   - `grep -n "TODO" "$f"` passes (`-n` is an allowed flag)

Every leaf is safe, so the entire command is auto-approved without over-extending permissions to the agent.

## Interaction with approved commands

safe-chains runs as a pre-hook. If it approves, Claude Code skips the prompt. If it doesn't recognize the command, Claude Code's normal permission flow takes over (checking your `Bash(...)` patterns in settings, or prompting).

Where this gets interesting is chained commands. Claude Code matches approved patterns against the full command string. If you approved `Bash(cargo test:*)` and Claude runs `cargo test && ./generate-docs.sh`, Claude Code won't match, since the full string isn't just `cargo test`.

safe-chains splits the chain and checks each segment independently. `cargo test` passes built-in rules. `./generate-docs.sh` matches `Bash(./generate-docs.sh:*)` from your settings. Both segments covered, chain auto-approved.

Once safe-chains is handling your safe commands, most of your existing approved patterns are redundant. Strip them down to project-specific scripts and tools safe-chains doesn't know about, or write a [Custom Command](custom-commands.md) for those scripts and let safe-chains validate them with the same flag-level rules it applies to built-ins. See [Cleaning up approved commands](configuration.md#cleaning-up-approved-commands).

For example, given `cargo test && npm run build && ./generate-docs.sh`:

- `cargo test` passes built-in rules
- `npm run build` matches `Bash(npm run:*)` from settings
- `./generate-docs.sh` matches `Bash(./generate-docs.sh:*)` from settings
