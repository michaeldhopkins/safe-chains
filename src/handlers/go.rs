use crate::parse::{Token, WordSet};

static GO_SAFE: WordSet =
    WordSet::new(&["--version", "build", "doc", "env", "list", "test", "version", "vet"]);

pub fn is_safe_go(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && GO_SAFE.contains(&tokens[1])
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![CommandDoc::wordset("go", &GO_SAFE)]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        go_version: "go version",
        go_env: "go env GOPATH",
        go_env_bare: "go env",
        go_list: "go list ./...",
        go_vet: "go vet ./...",
        go_test: "go test ./...",
        go_test_verbose: "go test -v ./...",
        go_build: "go build ./...",
        go_doc: "go doc fmt.Println",
        go_version_flag: "go --version",
    }

    denied! {
        go_run_denied: "go run main.go",
        go_install_denied: "go install golang.org/x/tools/...@latest",
        go_get_denied: "go get golang.org/x/tools",
        go_clean_denied: "go clean",
        go_generate_denied: "go generate ./...",
        go_mod_tidy_denied: "go mod tidy",
        bare_go_denied: "go",
    }
}
