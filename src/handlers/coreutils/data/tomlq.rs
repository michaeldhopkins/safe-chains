use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static TOMLQ_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--compact-output", "--exit-status", "--help",
        "--null-input", "--raw-input", "--raw-output",
        "--slurp", "--sort-keys", "--tab", "--toml-output",
        "--version",
        "-C", "-M", "-R", "-S", "-V", "-c", "-e", "-h", "-n", "-r", "-s", "-t",
    ]),
    valued: WordSet::flags(&[
        "--arg", "--argjson", "--indent", "-f",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "tomlq", policy: &TOMLQ_POLICY, level: SafetyLevel::Inert, url: "https://github.com/kislyuk/yq", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        tomlq_filter: "tomlq '.package.name' Cargo.toml",
        tomlq_toml_output: "tomlq -t '.deps' Cargo.toml",
        tomlq_raw: "tomlq -r '.package.version' Cargo.toml",
    }

    denied! {
        tomlq_bare_denied: "tomlq",
        tomlq_inplace_denied: "tomlq -i '.key = \"val\"' file.toml",
        tomlq_inplace_long_denied: "tomlq --in-place '.key = \"val\"' file.toml",
    }
}
