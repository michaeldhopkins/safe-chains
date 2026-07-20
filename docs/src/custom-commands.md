# Custom Commands

safe-chains ships definitions for hundreds of tools. To add an in-house CLI or disallow a built-in command, write a TOML file in one of two places:

- **`~/.config/safe-chains.toml`**: applies everywhere, trusted automatically.
- **`.safe-chains.toml`** in a repo (or any parent directory): applies to that project, once you trust the directory.

When a command is defined in both, a trusted project definition overrides the user-level one.

## Project files must be trusted

A repo's `.safe-chains.toml` sits inside the code safe-chains checks, and whatever edits the repo can edit that file. safe-chains ignores it until you pin the directory in your user config:

```toml
# ~/.config/safe-chains.toml
[[trusted]]
path = "/Users/you/work/acme"
sha256 = "9f2b…"   # shasum -a 256 acme/.safe-chains.toml
```

safe-chains honors the project file only while its contents match `sha256`. A change to the file un-trusts it until you review the change and trust it again.

Home directory level config (`~/.config/safe-chains.toml` and `~/.claude/settings.json`) is trusted without pinning. Be careful granting access to these directories.

Granting trust requires a manual edit for now.

## Add a tool safe-chains doesn't know

```toml
# .safe-chains.toml

[[command]]
name = "myco"
description = "MyCo internal CLI"
url = "https://wiki.myco/cli"
bare_flags = ["--help", "--version", "-h", "-v"]

[[command.sub]]
name = "deploy"
level = "SafeWrite"
standalone = ["--help", "--dry-run", "-h"]
valued = ["--env", "--region"]
max_positional = 1

[[command.sub]]
name = "status"
standalone = ["--help", "--watch", "-h", "-w"]
valued = ["--env"]

[[command.sub]]
name = "logs"
standalone = ["--help", "--follow", "-f"]
valued = ["--service", "--since", "--lines"]
```

This allows `myco --help`, `myco deploy --dry-run staging`, `myco status --env prod`, and so on. Anything outside the listed flags or subcommands is denied.

The schema mirrors the built-in TOMLs. Every field documented in [`commands/SAMPLE.toml`](https://github.com/michaeldhopkins/safe-chains/blob/main/commands/SAMPLE.toml) works in custom files.

## A shell script

```toml
# ~/.config/safe-chains.toml

[[command]]
name = "generate-docs.sh"
bare = true
max_positional = 0
```

Names match the command's basename. `./generate-docs.sh`, `bin/generate-docs.sh`, and `generate-docs.sh` all look up the same entry.

## Disallow a built-in command for this project

```toml
[[command]]
name = "gh"
deny = true
```

Three lines and `gh` is denied in this project: bare invocation, subcommands, and every flag.

## Generate one with an AI

Paste your tool's `--help` output and this prompt into Claude or another LLM:

> Generate a safe-chains custom command definition. Use the schema in <https://github.com/michaeldhopkins/safe-chains/blob/main/commands/SAMPLE.toml>. Output a single TOML block I can paste into `.safe-chains.toml`. Cover read-only and idempotent subcommands; omit destructive ones.

If you paste it into a repo's `.safe-chains.toml`, pin the directory afterward (see [Project files must be trusted](#project-files-must-be-trusted)).

## Skipping custom files: `SAFE_CHAINS_NO_LOCAL`

Set `SAFE_CHAINS_NO_LOCAL=1` to skip the project-local walk and the user-level lookup. Two reasons:

**Debugging.** If a custom definition might be interfering with a command, run with the bypass to compare against built-in behavior:

```sh
SAFE_CHAINS_NO_LOCAL=1 safe-chains "your command"
```

**Slow filesystems.** Each invocation makes a few `stat()` calls walking up from the current directory. On a local SSD this is microseconds. On network mounts (NFS, corporate file shares), WSL1 with files on the Windows side, or under aggressive antivirus, each `stat()` can cost tens of milliseconds. If you don't use custom commands and you're on one of those filesystems, export the variable in your shell init:

```sh
export SAFE_CHAINS_NO_LOCAL=1
```

Now safe-chains skips the lookup entirely.
