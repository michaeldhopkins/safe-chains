use crate::command::{CommandDef, SubDef};
use crate::verdict::{SafetyLevel, Verdict};
use crate::parse::{Token, WordSet};
use crate::policy::{FlagPolicy, FlagStyle};

static SWIFT_BUILD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--enable-code-coverage", "--help", "--show-bin-path",
        "--skip-update", "--static-swift-stdlib", "--verbose",
        "-h", "-v",
    ]),
    valued: WordSet::flags(&[
        "--arch", "--build-path", "--configuration", "--jobs",
        "--package-path", "--product", "--sanitize", "--swift-sdk",
        "--target", "--triple",
        "-c", "-j",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SWIFT_TEST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--enable-code-coverage", "--help", "--list-tests", "--parallel",
        "--show-codecov-path", "--skip-build", "--skip-update",
        "--verbose",
        "-h", "-l", "-v",
    ]),
    valued: WordSet::flags(&[
        "--arch", "--build-path", "--configuration", "--filter",
        "--jobs", "--num-workers", "--package-path", "--sanitize",
        "--skip-tests", "--swift-sdk", "--target", "--triple",
        "--xunit-output",
        "-c", "-j",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SWIFT_PACKAGE_DESCRIBE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&["--package-path", "--type"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SWIFT_PACKAGE_DUMP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&["--package-path"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SWIFT_PACKAGE_SHOW_DEPS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&["--format", "--package-path"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static SWIFT: CommandDef = CommandDef {
    name: "swift",
    subs: &[
        SubDef::Policy { name: "build", policy: &SWIFT_BUILD_POLICY, level: SafetyLevel::SafeWrite },
        SubDef::Nested { name: "package", subs: &[
            SubDef::Policy { name: "describe", policy: &SWIFT_PACKAGE_DESCRIBE_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "dump-package", policy: &SWIFT_PACKAGE_DUMP_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "show-dependencies", policy: &SWIFT_PACKAGE_SHOW_DEPS_POLICY, level: SafetyLevel::Inert },
        ]},
        SubDef::Policy { name: "test", policy: &SWIFT_TEST_POLICY, level: SafetyLevel::SafeRead },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://www.swift.org/documentation/swift-compiler/",
    aliases: &[],
};

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    SWIFT.dispatch(cmd, tokens)
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![SWIFT.to_doc()]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        swift_version: "swift --version",
        swift_test: "swift test",
        swift_test_verbose: "swift test --verbose",
        swift_test_filter: "swift test --filter MyTests",
        swift_test_parallel: "swift test --parallel",
        swift_test_list: "swift test --list-tests",
        swift_test_config: "swift test --configuration release",
        swift_build: "swift build",
        swift_build_verbose: "swift build --verbose",
        swift_build_config: "swift build --configuration release",
        swift_build_show_bin: "swift build --show-bin-path",
        swift_build_arch: "swift build --arch arm64",
        swift_package_describe: "swift package describe",
        swift_package_describe_type: "swift package describe --type json",
        swift_package_dump_package: "swift package dump-package",
        swift_package_show_dependencies: "swift package show-dependencies",
        swift_package_show_deps_format: "swift package show-dependencies --format json",
    }

    denied! {
        swift_package_bare_denied: "swift package",
    }
}
