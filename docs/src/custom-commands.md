# Custom Commands

safe-chains ships definitions for hundreds of tools. If your project uses an in-house CLI it doesn't recognize, or you want to disallow a built-in command for a specific project, drop a TOML file in either of these locations:

- **`.safe-chains.toml`** in your repo root (or any parent directory)
- **`~/.config/safe-chains.toml`** for definitions that apply across all your projects

When a name appears in more than one place, the more local definition wins: project beats user, user beats built-in.

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

The schema mirrors the built-in TOMLs — every field documented in [`commands/SAMPLE.toml`](https://github.com/michaeldhopkins/safe-chains/blob/main/commands/SAMPLE.toml) works in custom files.

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

Three lines and `gh` is denied in this project — bare invocation, subcommands, and every flag.

## Generate one with an AI

Paste your tool's `--help` output and this prompt into Claude or another LLM:

> Generate a safe-chains custom command definition. Use the schema in <https://github.com/michaeldhopkins/safe-chains/blob/main/commands/SAMPLE.toml>. Output a single TOML block I can paste into `.safe-chains.toml`. Cover read-only and idempotent subcommands; omit destructive ones.

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
