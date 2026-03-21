use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static PERIPHERY_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static PERIPHERY_SCAN_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--quiet", "--skip-build", "--strict", "--verbose",
        "-h",
    ]),
    valued: WordSet::flags(&[
        "--config", "--format", "--index-store-path",
        "--project", "--schemes", "--targets",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static PERIPHERY: CommandDef = CommandDef {
    name: "periphery",
    subs: &[
        SubDef::Policy { name: "scan", policy: &PERIPHERY_SCAN_POLICY, level: SafetyLevel::SafeRead },
        SubDef::Policy { name: "version", policy: &PERIPHERY_BARE_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://github.com/peripheryapp/periphery",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        periphery_scan: "periphery scan",
        periphery_scan_config: "periphery scan --config .periphery.yml",
        periphery_scan_project: "periphery scan --project MyApp.xcodeproj",
        periphery_scan_schemes: "periphery scan --schemes MyApp",
        periphery_scan_targets: "periphery scan --targets MyApp",
        periphery_scan_format: "periphery scan --format json",
        periphery_scan_quiet: "periphery scan --quiet",
        periphery_scan_verbose: "periphery scan --verbose",
        periphery_scan_strict: "periphery scan --strict",
        periphery_scan_skip_build: "periphery scan --skip-build",
        periphery_scan_combined: "periphery scan --skip-build --format json",
        periphery_scan_index_store: "periphery scan --index-store-path /tmp/index",
        periphery_version: "periphery version",
    }

    denied! {
        periphery_bare_denied: "periphery",
        periphery_clear_cache_denied: "periphery clear-cache",
    }
}
