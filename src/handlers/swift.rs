use crate::command::{CommandDef, SubDef};
use crate::parse::{Segment, Token, WordSet};
use crate::policy::{FlagPolicy, FlagStyle};

static SWIFT_BUILD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--enable-code-coverage", "--show-bin-path",
        "--skip-update", "--static-swift-stdlib", "--verbose",
    ]),
    standalone_short: b"v",
    valued: WordSet::flags(&[
        "--arch", "--build-path", "--configuration", "--jobs",
        "--package-path", "--product", "--sanitize", "--swift-sdk",
        "--target", "--triple",
    ]),
    valued_short: b"cj",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SWIFT_TEST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--enable-code-coverage", "--list-tests", "--parallel",
        "--show-codecov-path", "--skip-build", "--skip-update",
        "--verbose",
    ]),
    standalone_short: b"lv",
    valued: WordSet::flags(&[
        "--arch", "--build-path", "--configuration", "--filter",
        "--jobs", "--num-workers", "--package-path", "--sanitize",
        "--skip-tests", "--swift-sdk", "--target", "--triple",
        "--xunit-output",
    ]),
    valued_short: b"cj",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SWIFT_PACKAGE_DESCRIBE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&["--package-path", "--type"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SWIFT_PACKAGE_DUMP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&["--package-path"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SWIFT_PACKAGE_SHOW_DEPS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&["--format", "--package-path"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static SWIFT: CommandDef = CommandDef {
    name: "swift",
    subs: &[
        SubDef::Policy { name: "build", policy: &SWIFT_BUILD_POLICY },
        SubDef::Nested { name: "package", subs: &[
            SubDef::Policy { name: "describe", policy: &SWIFT_PACKAGE_DESCRIBE_POLICY },
            SubDef::Policy { name: "dump-package", policy: &SWIFT_PACKAGE_DUMP_POLICY },
            SubDef::Policy { name: "show-dependencies", policy: &SWIFT_PACKAGE_SHOW_DEPS_POLICY },
        ]},
        SubDef::Policy { name: "test", policy: &SWIFT_TEST_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://www.swift.org/documentation/swift-compiler/",
};

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    SWIFT.dispatch(cmd, tokens, is_safe)
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
