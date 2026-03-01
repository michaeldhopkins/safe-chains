use crate::parse::{Token, WordSet};

static DOTNET_SAFE: WordSet = WordSet::new(&[
    "--info", "--list-runtimes", "--list-sdks", "--version",
    "build", "list", "test",
]);

pub fn is_safe_dotnet(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && DOTNET_SAFE.contains(&tokens[1])
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![CommandDoc::wordset("dotnet", &DOTNET_SAFE)]
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
        dotnet_test: "dotnet test",
        dotnet_list: "dotnet list package",
    }

    denied! {
        dotnet_run_denied: "dotnet run",
        dotnet_new_denied: "dotnet new console",
        dotnet_add_denied: "dotnet add package Newtonsoft.Json",
        dotnet_publish_denied: "dotnet publish",
        dotnet_clean_denied: "dotnet clean",
        dotnet_restore_denied: "dotnet restore",
        bare_dotnet_denied: "dotnet",
    }
}
