# Your Rules

safe-chains reads rules from three files: `~/.config/safe-chains.toml`, a trusted `.safe-chains.toml` in the project, and the read approvals in `~/.claude/settings.json` (only when Claude Code runs it). The `config` commands show those rules and change the first file without opening it.

## What would change a refusal

`--explain` ends with what would change its answer. There are three possibilities:

```console
$ safe-chains --explain "deploytool status --env prod"
...
Fix: add an entry for `deploytool`:
  safe-chains config add 'deploytool status --env prod'

$ safe-chains --explain "tee ~/notes/today.md"
...
Fix: grant write access to `~/notes`:
  safe-chains config grant ~/notes --write

$ safe-chains --explain "git push --force"
...
Fix: none in safe-chains' config. `git` is built in, and a built-in rule refused an
option or a subcommand used here. No entry or grant can change that. A person
approves the command each time it runs.
```

1. **Add an entry**, when safe-chains has no entry for the command.
2. **Grant a path**, when a command safe-chains knows reached a folder outside the places it approves by default. The grant is offered only if it would get the command approved.
3. **Nothing**, when a built-in rule refused an option, a subcommand or a protected path. A person approves the command each time it runs.

When a hook refuses a command, its message says that `--explain` will tell you which of these applies.

## Listing the rules

```bash
safe-chains config list
```

prints each rule under the file it came from, including whether a project's `.safe-chains.toml` is trusted.

## Changing them

```bash
safe-chains config add "deploytool status --env prod"
safe-chains config grant ~/notes --write
safe-chains config remove command deploytool
safe-chains config remove grant ~/notes
```

`config add` writes an entry for a command safe-chains has no entry for. The entry covers the command used the way you wrote it: the same subcommand and flags, and up to as many other arguments. `deploytool status --env prod` approves `deploytool status --env staging`, but not `deploytool destroy`. `--level inert`, `safe-read` or `safe-write` (the default) sets how much the command may do.

`config grant` adds a `[[grant]]` with `--read`, `--write`, or both. Granting a path that already has a grant adds to it.

Each change keeps the rest of the file as it was, comments included, and takes effect from the next command.

## What they refuse

These commands will not:

- replace a built-in command's rules, or add an entry for a command that runs an AI coding agent, since such an agent can change its own approval settings;
- grant a credential store such as `~/.ssh`, safe-chains' own config, the files that decide who may log in, or a device;
- grant a place whose contents run later without anyone asking: a coding agent's settings, a shell or login startup file, `~/.gitconfig`, a launch agent folder, or a folder on PATH, or a folder above one of those;
- grant `/`, your home folder, or a path with a wildcard;
- add an entry for a program named by its path, or for a command whose subcommand comes after a flag, since either would approve more than you wrote;
- write a file safe-chains could not load.

A grant on a credential store can still be written into `~/.config/safe-chains.toml` by hand.

## Nobody approves themselves

safe-chains never auto-approves `config add`, `config remove`, `config grant` or `mcp`. An agent that runs one gets your harness's normal approval prompt, or a refusal on a harness that blocks. `config list` is approved like any other read.
