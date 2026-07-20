# `safe-chains`

A command safety checker that auto-allows safe bash commands without prompting. Works as a Claude Code pre-hook, a CLI tool, or an OpenCode plugin.

safe-chains knows 450+ commands. For each one it validates specific subcommands and flags: allowing `git log` but not `git push`, allowing `sed 's/foo/bar/'` but not `sed -i`. Commands in piped chains, `&&`, and `;` sequences are each validated independently. Compound commands (`for`, `while`, `if`) are parsed recursively.

[Documentation and supported command reference](https://www.michaeldhopkins.com/docs/safe-chains/)

## Install

```bash
brew install michaeldhopkins/tap/safe-chains
```

Or `cargo install safe-chains`, or download a binary from [GitHub Releases](https://github.com/michaeldhopkins/safe-chains/releases/latest).

## Configure

```bash
safe-chains --setup
```

This adds the Claude Code pre-hook to `~/.claude/settings.json`. Restart Claude Code to activate. See the [documentation](https://www.michaeldhopkins.com/docs/safe-chains/configuration.html) for manual setup and OpenCode configuration.

## Usage

With the hook configured, safe-chains runs automatically. No interaction needed. Safe commands are approved, everything else goes through the normal permission prompt.

As a CLI tool:

```bash
safe-chains "ls -la | head -5"    # exit 0 = safe
safe-chains "rm -rf /"            # exit 1 = unsafe
```

Use `--level` to set a threshold; only commands at or below it auto-approve. The levels, from
most locked down to most open, are `paranoid` (barely reads), `reader` (reads your project),
`editor` (edits it, no delete/run/network), `developer` (the everyday dev default, runs your
project, deletes your own files), the two admin flavors `local-admin` (runs this machine) and
`network-admin` (operates your remotes), and `yolo` (no limits except `rm -rf /`). Default:
`developer`.

```bash
safe-chains --level paranoid "cargo test"   # exit 1 (reads the tree, above paranoid)
safe-chains --level reader "cargo test"     # exit 0
```

The legacy names `inert` / `safe-read` / `safe-write` still work. They map to
`paranoid` / `reader` / `developer` and print a one-line notice.

## Custom commands

Drop a `.safe-chains.toml` in your repo (or `~/.config/safe-chains.toml` for all projects) to add tools safe-chains doesn't know about, or to lock down a built-in command. See the [Custom Commands docs](https://www.michaeldhopkins.com/docs/safe-chains/custom-commands.html).

## Contributing

Found a safe command safe-chains should support? [Submit an issue.](https://github.com/michaeldhopkins/safe-chains/issues/new?template=command-request.yml)

See the [documentation](https://www.michaeldhopkins.com/docs/safe-chains/contributing.html) for how to add commands.

---

Copyright 2026 Michael Hopkins
