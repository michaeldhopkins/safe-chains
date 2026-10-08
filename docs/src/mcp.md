# MCP Server

`safe-chains mcp` serves the same checks and rule changes as the CLI, as MCP tools over stdin and stdout. It is for agents that call tools rather than run commands, and for smaller models, which pick the right typed tool more reliably than they compose a shell command.

| Tool | What it does | Changes anything |
|------|--------------|------------------|
| `check_command` | Whether a command would run without asking, and what would change that | No |
| `list_rules` | Every rule safe-chains reads from a file, with the file it came from | No |
| `add_command_entry` | Same as `safe-chains config add` | Yes |
| `grant_path` | Same as `safe-chains config grant` | Yes |
| `remove_rule` | Same as `safe-chains config remove` | Yes |

Each tool runs the `safe-chains` binary, so the server and the CLI always agree, and a change applies from the next call on.

## Never let a client approve the editing tools by itself

`add_command_entry`, `grant_path` and `remove_rule` change what runs without asking. A client that runs them without asking you lets an agent approve its own commands: grant itself a folder, add an entry for its own tool, then run it.

- Allow `check_command` and `list_rules` if you like. Never allow the server as a whole (`mcp__safe-chains`), the editing tools, or a mode that approves everything.
- The tools are marked destructive, which clients such as ChatGPT and Codex read as "ask first". Those marks are hints that a client may ignore.
- The server refuses to edit when it can see a client set to approve that without asking: an allow rule for an editing tool or for the whole server (under any name it is registered as) in Claude Code's settings files, `~/.claude.json` or the managed settings, `defaultMode: "bypassPermissions"`, a Gemini CLI server entry with `trust: true`, or `approval_policy = "never"` in `~/.codex/config.toml`. It also refuses when one of those files is there but cannot be read.
- It cannot see a command-line flag that skips every prompt, or a client whose settings it does not know. Those are yours to avoid.

The editing tools refuse what `safe-chains config` refuses. See [Your Rules](rules.md).

## Setup

Claude Code, for every project:

```bash
claude mcp add --scope user safe-chains -- safe-chains mcp
```

Then allow only the read-only tools, in `~/.claude/settings.json`:

```json
"permissions": {
  "allow": ["mcp__safe-chains__check_command", "mcp__safe-chains__list_rules"]
}
```

Codex, in `~/.codex/config.toml`:

```toml
[mcp_servers.safe-chains]
command = "safe-chains"
args = ["mcp"]
```

Any other client: run `safe-chains` with the argument `mcp` as a stdio server.
