# Adding Commands

Most commands are defined as TOML in the `commands/` directory. See [`commands/SAMPLE.toml`](https://github.com/michaeldhopkins/safe-chains/blob/main/commands/SAMPLE.toml) for a documented reference of every supported field.

## Steps

1. Add the command to the appropriate `commands/*.toml` file (or create a new one)
2. If you created a new file, register it in `src/registry.rs` via `include_str!`
3. Add the command name to `HANDLED_CMDS` in `src/handlers/mod.rs`
4. Run `cargo test` and `cargo clippy -- -D warnings`
5. Run `./generate-docs.sh` to regenerate documentation
6. Run `cargo install --path .` to update the installed binary

For commands that need custom validation logic, add a Rust handler in `src/handlers/` and reference it with `handler = "name"` in the TOML.

{{#include includes/cta-new-command.md}}

## Reporting vulnerabilities

{{#include includes/cta-vulnerability.md}}
