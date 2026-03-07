use crate::command::{CommandDef, SubDef};
use crate::parse::{Segment, Token, WordSet};
use crate::policy::{FlagPolicy, FlagStyle};

static DOTNET_BUILD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--force", "--no-dependencies", "--no-incremental",
        "--no-restore", "--nologo", "--self-contained",
        "--tl", "--use-current-runtime",
    ]),
    valued: WordSet::flags(&[
        "--arch", "--artifacts-path", "--configuration", "--framework",
        "--os", "--output", "--property", "--runtime", "--source",
        "--verbosity", "--version-suffix",
        "-a", "-c", "-f", "-o", "-p", "-r", "-s", "-v",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOTNET_TEST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--blame", "--blame-crash", "--blame-hang", "--force",
        "--list-tests", "--no-build", "--no-dependencies",
        "--no-restore", "--nologo",
    ]),
    valued: WordSet::flags(&[
        "--arch", "--artifacts-path", "--blame-crash-collect-always",
        "--blame-crash-dump-type", "--blame-hang-dump-type",
        "--blame-hang-timeout", "--collect", "--configuration",
        "--diag", "--environment", "--filter", "--framework",
        "--logger", "--os", "--output", "--property",
        "--results-directory", "--runtime", "--settings",
        "--test-adapter-path", "--verbosity",
        "-a", "-c", "-d", "-e", "-f", "-l", "-o", "-r", "-s", "-v",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOTNET_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--deprecated", "--highest-minor", "--highest-patch",
        "--include-prerelease", "--include-transitive", "--outdated",
        "--vulnerable",
    ]),
    valued: WordSet::flags(&[
        "--config", "--format", "--framework", "--source", "--verbosity",
        "-v",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static DOTNET: CommandDef = CommandDef {
    name: "dotnet",
    subs: &[
        SubDef::Policy { name: "build", policy: &DOTNET_BUILD_POLICY },
        SubDef::Policy { name: "list", policy: &DOTNET_LIST_POLICY },
        SubDef::Policy { name: "test", policy: &DOTNET_TEST_POLICY },
    ],
    bare_flags: &["--info", "--list-runtimes", "--list-sdks"],
    help_eligible: true,
    url: "https://learn.microsoft.com/en-us/dotnet/core/tools/",
};

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    DOTNET.dispatch(cmd, tokens, is_safe)
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![DOTNET.to_doc()]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        dotnet_version: "dotnet --version",
        dotnet_info: "dotnet --info",
        dotnet_list_sdks: "dotnet --list-sdks",
        dotnet_list_runtimes: "dotnet --list-runtimes",
        dotnet_build: "dotnet build",
        dotnet_build_config: "dotnet build --configuration Release",
        dotnet_build_framework: "dotnet build --framework net8.0",
        dotnet_build_nologo: "dotnet build --nologo",
        dotnet_build_verbosity: "dotnet build --verbosity quiet",
        dotnet_test: "dotnet test",
        dotnet_test_filter: "dotnet test --filter FullyQualifiedName~MyTest",
        dotnet_test_logger: "dotnet test --logger trx",
        dotnet_test_no_build: "dotnet test --no-build",
        dotnet_test_blame: "dotnet test --blame",
        dotnet_test_verbosity: "dotnet test --verbosity minimal",
        dotnet_list: "dotnet list package",
        dotnet_list_outdated: "dotnet list package --outdated",
        dotnet_list_vulnerable: "dotnet list package --vulnerable",
        dotnet_list_deprecated: "dotnet list package --deprecated",
        dotnet_list_transitive: "dotnet list package --include-transitive",
    }
}
