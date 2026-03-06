use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

static DOTNET_BUILD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--force", "--no-dependencies", "--no-incremental",
        "--no-restore", "--nologo", "--self-contained",
        "--tl", "--use-current-runtime",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "--arch", "--artifacts-path", "--configuration", "--framework",
        "--os", "--output", "--property", "--runtime", "--source",
        "--verbosity", "--version-suffix",
    ]),
    valued_short: b"acfoprsv",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOTNET_TEST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--blame", "--blame-crash", "--blame-hang", "--force",
        "--list-tests", "--no-build", "--no-dependencies",
        "--no-restore", "--nologo",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "--arch", "--artifacts-path", "--blame-crash-collect-always",
        "--blame-crash-dump-type", "--blame-hang-dump-type",
        "--blame-hang-timeout", "--collect", "--configuration",
        "--diag", "--environment", "--filter", "--framework",
        "--logger", "--os", "--output", "--property",
        "--results-directory", "--runtime", "--settings",
        "--test-adapter-path", "--verbosity",
    ]),
    valued_short: b"acdeflorsv",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOTNET_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--deprecated", "--highest-minor", "--highest-patch",
        "--include-prerelease", "--include-transitive", "--outdated",
        "--vulnerable",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "--config", "--format", "--framework", "--source", "--verbosity",
    ]),
    valued_short: b"v",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_dotnet(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    match tokens[1].as_str() {
        "--info" | "--list-runtimes" | "--list-sdks" => tokens.len() == 2,
        "build" => policy::check(&tokens[1..], &DOTNET_BUILD_POLICY),
        "test" => policy::check(&tokens[1..], &DOTNET_TEST_POLICY),
        "list" => policy::check(&tokens[1..], &DOTNET_LIST_POLICY),
        _ => false,
    }
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "dotnet" => Some(is_safe_dotnet(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![CommandDoc::handler("dotnet",
        "Subcommands: build, list, test. Info flags: --info, --list-runtimes, --list-sdks. \
        ")]
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

    denied! {
        dotnet_run_denied: "dotnet run",
        dotnet_new_denied: "dotnet new console",
        dotnet_add_denied: "dotnet add package Newtonsoft.Json",
        dotnet_publish_denied: "dotnet publish",
        dotnet_clean_denied: "dotnet clean",
        dotnet_restore_denied: "dotnet restore",
        bare_dotnet_denied: "dotnet",
        dotnet_build_unknown_denied: "dotnet build --unknown",
        dotnet_test_unknown_denied: "dotnet test --unknown-flag",
        dotnet_list_unknown_denied: "dotnet list package --unknown",
    }
}
