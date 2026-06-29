# Trusted customization

Status: accepted, not yet implemented (2026-06-29). Internal design note — not part of the rendered book.

## Problem

safe-chains' security rests on two things the agent can't change: the hook is wired into the harness config, and the binary's verdict is deterministic. But safe-chains also reads trust-widening config from the **project tree**, which the agent edits as ordinary work:

- `registry/custom.rs` loads `<repo>/.safe-chains.toml` (via `find_repo_custom`, walking up from cwd). This can *redefine a built-in command's safety level* — e.g. mark `curl` Inert — not just add patterns. The sharpest edge.
- `allowlist.rs` reads `permissions.allow` / `approved_commands` from `<repo>/.claude/settings.json` and `settings.local.json`.

So an agent can self-grant approval for any command by writing the file first, then running the command. Latent today only because models don't exploit it.

## Model

Trust is keyed to **home config vs. the project tree**, not to a filename.

- **Home config** (`~/.config/safe-chains.toml`, `~/.claude/settings.json`): trust root. The agent edits it only with the user's permission. Read and honored as-is, never hashed.
- **Project tree** (`<repo>/.safe-chains.toml`): the agent edits the repo freely. Honored only when the directory is pinned in home config *and* the file's hash matches the pin.

The hash is required because, without it, "trust this directory" means "honor whatever its `.safe-chains.toml` ever says" — an agent in an already-trusted repo could rewrite the file. The pin scopes trust to reviewed content.

Trust is granted only by hand-editing home config. No `safe-chains trust` subcommand — such a command would be agent-invocable, defeating the point.

### Assumption (documented in security.md)

The boundary holds only while home config and the hook registration are outside the agent's no-permission write scope. An agent that can rewrite home config can remove the hook; no allowlist scheme survives that.

## Config schema (home `~/.config/safe-chains.toml`)

```toml
[[trusted]]
path = "/abs/path/to/repo"
sha256 = "…"   # sha256 of <repo>/.safe-chains.toml; `shasum -a 256`
```

- `path` is an absolute directory path. A `<repo>/.safe-chains.toml` is honored only if `<repo>` (or an ancestor that owns the file) matches a `path`.
- `sha256` is required. Mismatch → the project file is ignored.

## Code changes

1. **`allowlist.rs` (`Matcher::load_with_project_dir`)** — stop reading `<project>/.claude/settings.json` and `settings.local.json`. Read `.claude/settings.json` only from `$HOME`. The harness applies its own project settings directly, so this costs the user nothing.
2. **`registry/custom.rs`** — `find_repo_custom()` still locates `<repo>/.safe-chains.toml`, but `apply_custom` honors it only if its directory is pinned in the user config and the file's sha256 matches. User-level `~/.config/safe-chains.toml` is unchanged (trust root).
3. **Trusted-dir loading** — parse `[[trusted]]` from `~/.config/safe-chains.toml`. Keep it separate from the command-definition parse so the trust list itself can never come from a project file. `TomlFile.command` is defaulted so a user config holding only `[[trusted]]` parses cleanly.

An untrusted or hash-mismatched project file is **silently ignored** — no per-invocation notice. The hook runs on every command, so a notice would be noise; surfacing it belongs in a future status/doctor view, not the hot path.

## Scoped commits

1. `docs` — the model in `custom-commands.md` / `security.md` and this note.
2. `feat` — stop reading project `.claude/settings.json`; read home only. (behavior change)
3. `feat` — parse `[[trusted]]`; gate `<repo>/.safe-chains.toml` on pin + sha256. (the core fix)

## Tests

- Home `~/.claude/settings.json`: honored, no pin needed; a project `.claude/settings.json` is not read.
- Project `.safe-chains.toml`: ignored unless its dir is pinned **and** sha256 matches.
- Hash mismatch (agent rewrote the file after pinning) → ignored.
- `load_toml` tolerates a config that is only `[[trusted]]`.
- **Adversarial:** a project `.safe-chains.toml` that redefines `curl`/`bash` as Inert is not honored unless pinned + matched — the registry-redefinition vector.

## Deferred

- Signed project allowlists (key the agent lacks) for committed/shared repos. Separate epic; does not change the default.
