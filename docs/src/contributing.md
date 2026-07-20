# Adding Commands

Most commands are defined as TOML in the `commands/` directory. See [`commands/SAMPLE.toml`](https://github.com/michaeldhopkins/safe-chains/blob/main/commands/SAMPLE.toml) for a documented reference of every supported field.

## Steps

1. Add the command to the appropriate `commands/*.toml` file (or create a new one; `build.rs` auto-discovers any `*.toml` under `commands/`)
2. Run `cargo test` and `cargo clippy -- -D warnings`
3. Run `./generate-docs.sh` to regenerate documentation
4. Run `cargo install --path .` to update the installed binary

For commands that need custom validation logic, add a Rust handler in `src/handlers/` and reference it with `handler = "name"` in the TOML.

{{#include includes/cta-new-command.md}}

## Reporting vulnerabilities

{{#include includes/cta-vulnerability.md}}
