use std::collections::HashSet;
use std::sync::LazyLock;

static GO_SAFE: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from(["version", "env", "list", "vet", "test", "build", "doc"])
});

pub fn is_safe_go(tokens: &[String]) -> bool {
    tokens.len() >= 2 && GO_SAFE.contains(tokens[1].as_str())
}

#[cfg(test)]
mod tests {
    use crate::is_safe;

    fn check(cmd: &str) -> bool {
        is_safe(cmd)
    }

    #[test]
    fn go_version() {
        assert!(check("go version"));
    }

    #[test]
    fn go_env() {
        assert!(check("go env GOPATH"));
    }

    #[test]
    fn go_env_bare() {
        assert!(check("go env"));
    }

    #[test]
    fn go_list() {
        assert!(check("go list ./..."));
    }

    #[test]
    fn go_vet() {
        assert!(check("go vet ./..."));
    }

    #[test]
    fn go_test() {
        assert!(check("go test ./..."));
    }

    #[test]
    fn go_test_verbose() {
        assert!(check("go test -v ./..."));
    }

    #[test]
    fn go_build() {
        assert!(check("go build ./..."));
    }

    #[test]
    fn go_doc() {
        assert!(check("go doc fmt.Println"));
    }

    #[test]
    fn go_run_denied() {
        assert!(!check("go run main.go"));
    }

    #[test]
    fn go_install_denied() {
        assert!(!check("go install golang.org/x/tools/...@latest"));
    }

    #[test]
    fn go_get_denied() {
        assert!(!check("go get golang.org/x/tools"));
    }

    #[test]
    fn go_clean_denied() {
        assert!(!check("go clean"));
    }

    #[test]
    fn go_generate_denied() {
        assert!(!check("go generate ./..."));
    }

    #[test]
    fn go_mod_tidy_denied() {
        assert!(!check("go mod tidy"));
    }

    #[test]
    fn bare_go_denied() {
        assert!(!check("go"));
    }
}
