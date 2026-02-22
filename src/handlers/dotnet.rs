use std::collections::HashSet;
use std::sync::LazyLock;

use crate::parse::Token;

static DOTNET_SAFE: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "--version",
        "--info",
        "--list-sdks",
        "--list-runtimes",
        "build",
        "test",
        "list",
    ])
});

pub fn is_safe_dotnet(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && DOTNET_SAFE.contains(tokens[1].as_str())
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocKind};
    vec![CommandDoc {
        name: "dotnet",
        kind: DocKind::Handler,
        description: "Allowed: --version, --info, --list-sdks, --list-runtimes, build, test, list.",
    }]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    #[test]
    fn dotnet_version() {
        assert!(check("dotnet --version"));
    }

    #[test]
    fn dotnet_info() {
        assert!(check("dotnet --info"));
    }

    #[test]
    fn dotnet_list_sdks() {
        assert!(check("dotnet --list-sdks"));
    }

    #[test]
    fn dotnet_list_runtimes() {
        assert!(check("dotnet --list-runtimes"));
    }

    #[test]
    fn dotnet_build() {
        assert!(check("dotnet build"));
    }

    #[test]
    fn dotnet_test() {
        assert!(check("dotnet test"));
    }

    #[test]
    fn dotnet_list() {
        assert!(check("dotnet list package"));
    }

    #[test]
    fn dotnet_run_denied() {
        assert!(!check("dotnet run"));
    }

    #[test]
    fn dotnet_new_denied() {
        assert!(!check("dotnet new console"));
    }

    #[test]
    fn dotnet_add_denied() {
        assert!(!check("dotnet add package Newtonsoft.Json"));
    }

    #[test]
    fn dotnet_publish_denied() {
        assert!(!check("dotnet publish"));
    }

    #[test]
    fn dotnet_clean_denied() {
        assert!(!check("dotnet clean"));
    }

    #[test]
    fn dotnet_restore_denied() {
        assert!(!check("dotnet restore"));
    }

    #[test]
    fn bare_dotnet_denied() {
        assert!(!check("dotnet"));
    }
}
