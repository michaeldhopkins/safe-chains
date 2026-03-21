use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static LINT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--list", "--quiet", "--show", "--version", "-V", "-h"]),
    valued: WordSet::flags(&["--check", "--config", "--disable", "--enable"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub static DEFS: &[FlatDef] = &[
    FlatDef {
        name: "lint",
        policy: &LINT_POLICY,
        level: SafetyLevel::Inert,
        url: "https://developer.android.com/studio/write/lint",
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
        lint_bare: "lint",
        lint_project: "lint /path/to/project",
        lint_list: "lint --list",
        lint_show: "lint --show",
        lint_quiet: "lint --quiet /path/to/project",
        lint_check: "lint --check MissingTranslation /path/to/project",
        lint_config: "lint --config lint.xml /path/to/project",
        lint_enable: "lint --enable UnusedResources /path/to/project",
        lint_disable: "lint --disable TooManyViews /path/to/project",
        lint_version: "lint --version",
        lint_help: "lint --help",
    }

    denied! {
        lint_apply_denied: "lint --apply-suggestions /path/to/project",
        lint_unknown_denied: "lint --unknown-flag",
    }
}
