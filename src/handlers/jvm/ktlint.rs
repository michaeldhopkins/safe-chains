use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static KTLINT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--color", "--color-name", "--help", "--relative", "--verbose", "--version", "-V", "-h"]),
    valued: WordSet::flags(&["--editorconfig", "--reporter"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub static DEFS: &[FlatDef] = &[
    FlatDef {
        name: "ktlint",
        policy: &KTLINT_POLICY,
        level: SafetyLevel::Inert,
        url: "https://pinterest.github.io/ktlint/latest/",
        aliases: &[],
    },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        ktlint_bare: "ktlint",
        ktlint_verbose: "ktlint --verbose",
        ktlint_relative: "ktlint --relative",
        ktlint_reporter: "ktlint --reporter=plain",
        ktlint_file: "ktlint src/main/kotlin/Foo.kt",
        ktlint_glob: "ktlint \"src/**/*.kt\"",
        ktlint_version: "ktlint --version",
        ktlint_help: "ktlint --help",
        ktlint_editorconfig: "ktlint --editorconfig=.editorconfig",
    }

    denied! {
        ktlint_format_denied: "ktlint -F",
        ktlint_format_long_denied: "ktlint --format",
        ktlint_unknown_denied: "ktlint --unknown-flag",
    }
}
