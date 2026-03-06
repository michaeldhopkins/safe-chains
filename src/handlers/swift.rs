use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

static SWIFT_BUILD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--enable-code-coverage", "--show-bin-path",
        "--skip-update", "--static-swift-stdlib", "--verbose",
    ]),
    standalone_short: b"v",
    valued: WordSet::new(&[
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
    standalone: WordSet::new(&[
        "--enable-code-coverage", "--list-tests", "--parallel",
        "--show-codecov-path", "--skip-build", "--skip-update",
        "--verbose",
    ]),
    standalone_short: b"lv",
    valued: WordSet::new(&[
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
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&["--package-path", "--type"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SWIFT_PACKAGE_DUMP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&["--package-path"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SWIFT_PACKAGE_SHOW_DEPS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&["--format", "--package-path"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_swift(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if tokens[1] == "build" {
        return policy::check(&tokens[1..], &SWIFT_BUILD_POLICY);
    }
    if tokens[1] == "test" {
        return policy::check(&tokens[1..], &SWIFT_TEST_POLICY);
    }
    if tokens[1] == "package" {
        if tokens.len() < 3 {
            return false;
        }
        let policy = match tokens[2].as_str() {
            "describe" => &SWIFT_PACKAGE_DESCRIBE_POLICY,
            "dump-package" => &SWIFT_PACKAGE_DUMP_POLICY,
            "show-dependencies" => &SWIFT_PACKAGE_SHOW_DEPS_POLICY,
            _ => return false,
        };
        return policy::check(&tokens[2..], policy);
    }
    false
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "swift" => Some(is_safe_swift(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![CommandDoc::handler("swift",
        "Subcommands: build, test, package describe, package dump-package, \
         package show-dependencies.")]
}

#[cfg(test)]
pub(super) const REGISTRY: &[super::CommandEntry] = &[
    super::CommandEntry::Subcommand { cmd: "swift", subs: &[
        super::SubEntry::Policy { name: "build" },
        super::SubEntry::Policy { name: "test" },
        super::SubEntry::Nested { name: "package", subs: &[
            super::SubEntry::Policy { name: "describe" },
            super::SubEntry::Policy { name: "dump-package" },
            super::SubEntry::Policy { name: "show-dependencies" },
        ]},
    ]},
];

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
        bare_swift_denied: "swift",
        swift_package_bare_denied: "swift package",
    }
}
