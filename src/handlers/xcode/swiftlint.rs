use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SWIFTLINT_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SWIFTLINT_LINT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--no-cache", "--quiet", "--strict"]),
    valued: WordSet::flags(&["--config", "--path", "--reporter"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SWIFTLINT_ANALYZE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--quiet", "--strict"]),
    valued: WordSet::flags(&["--compiler-log-path", "--config", "--path", "--reporter"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SWIFTLINT_RULES_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--disabled", "--enabled"]),
    valued: WordSet::flags(&["--config", "--reporter"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static SWIFTLINT: CommandDef = CommandDef {
    name: "swiftlint",
    subs: &[
        SubDef::Policy { name: "analyze", policy: &SWIFTLINT_ANALYZE_POLICY },
        SubDef::Policy { name: "lint", policy: &SWIFTLINT_LINT_POLICY },
        SubDef::Policy { name: "reporters", policy: &SWIFTLINT_BARE_POLICY },
        SubDef::Policy { name: "rules", policy: &SWIFTLINT_RULES_POLICY },
        SubDef::Policy { name: "version", policy: &SWIFTLINT_BARE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://github.com/realm/SwiftLint",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        swiftlint_lint: "swiftlint lint",
        swiftlint_lint_config: "swiftlint lint --config .swiftlint.yml",
        swiftlint_lint_path: "swiftlint lint --path Sources/",
        swiftlint_lint_reporter: "swiftlint lint --reporter json",
        swiftlint_lint_quiet: "swiftlint lint --quiet",
        swiftlint_lint_strict: "swiftlint lint --strict",
        swiftlint_lint_no_cache: "swiftlint lint --no-cache",
        swiftlint_analyze: "swiftlint analyze",
        swiftlint_analyze_compiler_log: "swiftlint analyze --compiler-log-path build.log",
        swiftlint_rules: "swiftlint rules",
        swiftlint_rules_enabled: "swiftlint rules --enabled",
        swiftlint_rules_disabled: "swiftlint rules --disabled",
        swiftlint_reporters: "swiftlint reporters",
        swiftlint_version: "swiftlint version",
    }

    denied! {
        swiftlint_bare_denied: "swiftlint",
        swiftlint_autocorrect_denied: "swiftlint autocorrect",
        swiftlint_fix_denied: "swiftlint fix",
    }
}
