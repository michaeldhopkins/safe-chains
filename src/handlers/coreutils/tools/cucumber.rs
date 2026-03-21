use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static CUCUMBER_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--backtrace", "--color", "--dry-run", "--expand",
        "--guess", "--help", "--i18n-keywords", "--i18n-languages",
        "--init", "--no-color", "--no-diff", "--no-multiline",
        "--no-snippets", "--no-source", "--no-strict",
        "--publish", "--publish-quiet", "--quiet",
        "--retry", "--snippets", "--strict", "--verbose", "--version",
        "--wip",
        "-V", "-b", "-d", "-e", "-h", "-q",
    ]),
    valued: WordSet::flags(&[
        "--ci-environment", "--format", "--format-options",
        "--language", "--lines", "--name", "--order",
        "--out", "--profile", "--require",
        "--require-module", "--retry", "--tags",
        "-f", "-i", "-l", "-n", "-o", "-p", "-r", "-t",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "cucumber", policy: &CUCUMBER_POLICY, level: SafetyLevel::Inert, url: "https://cucumber.io/docs/cucumber/api/#running-cucumber", aliases: &[] },
];
