use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static GRADLE_TASKS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--console", "--help", "--info", "--no-rebuild",
        "--quiet", "--stacktrace", "--warning-mode",
        "-h", "-q",
    ]),
    valued: WordSet::flags(&["--group"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GRADLE_DEPS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--console", "--help", "--info", "--no-rebuild",
        "--quiet", "--stacktrace", "--warning-mode",
        "-h", "-q",
    ]),
    valued: WordSet::flags(&["--configuration"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GRADLE_PROPS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--console", "--help", "--info", "--no-rebuild",
        "--quiet", "--stacktrace", "--warning-mode",
        "-h", "-q",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GRADLE_BUILD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--build-cache", "--configure-on-demand", "--console",
        "--continue", "--dry-run", "--help", "--info", "--no-build-cache",
        "--no-daemon", "--no-parallel", "--no-rebuild",
        "--parallel", "--profile", "--quiet", "--rerun-tasks",
        "--scan", "--stacktrace", "--warning-mode",
        "-h", "-q",
    ]),
    valued: WordSet::flags(&[
        "--exclude-task", "--max-workers",
        "-x",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GRADLE_SUBS: &[SubDef] = &[
    SubDef::Policy { name: "build", policy: &GRADLE_BUILD_POLICY, level: SafetyLevel::SafeWrite },
    SubDef::Policy { name: "check", policy: &GRADLE_BUILD_POLICY, level: SafetyLevel::SafeRead },
    SubDef::Policy { name: "dependencies", policy: &GRADLE_DEPS_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "properties", policy: &GRADLE_PROPS_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "tasks", policy: &GRADLE_TASKS_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "test", policy: &GRADLE_BUILD_POLICY, level: SafetyLevel::SafeRead },
];

pub(crate) static GRADLE: CommandDef = CommandDef {
    name: "gradle",
    subs: GRADLE_SUBS,
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://docs.gradle.org/current/userguide/command_line_interface.html",
    aliases: &["gradlew"],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        gradle_tasks: "gradle tasks",
        gradle_tasks_all: "gradle tasks --all",
        gradle_dependencies: "gradle dependencies",
        gradle_dependencies_config: "gradle dependencies --configuration implementation",
        gradle_properties: "gradle properties",
        gradle_version: "gradle --version",
        gradle_test: "gradle test",
        gradle_test_info: "gradle test --info",
        gradle_test_stacktrace: "gradle test --stacktrace",
        gradle_build: "gradle build",
        gradle_build_parallel: "gradle build --parallel",
        gradle_build_scan: "gradle build --scan",
        gradle_check: "gradle check",
        gradlew_test: "gradlew test",
        gradlew_version: "gradlew --version",
        gradlew_build_info: "gradlew build --info",
    }
}
