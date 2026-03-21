use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DETEKT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--build-upon-default-config", "--debug", "--help", "--parallel", "--version",
        "-V", "-h",
    ]),
    valued: WordSet::flags(&[
        "--baseline", "--classpath", "--config", "--config-resource",
        "--excludes", "--includes", "--input",
        "--jvm-target", "--language-version", "--plugins", "--report",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub static DEFS: &[FlatDef] = &[
    FlatDef {
        name: "detekt",
        policy: &DETEKT_POLICY,
        level: SafetyLevel::Inert,
        url: "https://detekt.dev/docs/gettingstarted/cli/",
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
        detekt_bare: "detekt",
        detekt_input: "detekt --input src/main/kotlin",
        detekt_config: "detekt --config detekt.yml",
        detekt_report: "detekt --report html:build/reports/detekt.html",
        detekt_baseline: "detekt --baseline baseline.xml",
        detekt_parallel: "detekt --parallel",
        detekt_debug: "detekt --debug",
        detekt_version: "detekt --version",
        detekt_help: "detekt --help",
        detekt_build_upon: "detekt --build-upon-default-config",
    }

    denied! {
        detekt_auto_correct_denied: "detekt --auto-correct",
        detekt_create_baseline_denied: "detekt --create-baseline",
        detekt_generate_config_denied: "detekt --generate-config",
        detekt_unknown_denied: "detekt --unknown-flag",
    }
}
