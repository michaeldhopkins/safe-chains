//! `mise set NAME=VALUE` stores the variable in a mise.toml that `mise activate` exports into every
//! later shell started below it. A one-off `NAME=VALUE cmd` prefix is judged for that one command;
//! a stored one also changes what later commands run, where they look for programs, and whom they
//! trust, so those names ask whatever the value. The registry-wide property over the list in
//! `envvars.toml` is `envvars::stored::tests::every_stored_name_is_refused_whatever_the_value`.

use safe_chains::pathctx::{PathCtx, enter};

fn approved(command: &str) -> bool {
    let root = env!("CARGO_MANIFEST_DIR").to_string();
    let _ctx = enter(PathCtx { cwd: Some(root.clone()), root: Some(root), session_id: None });
    safe_chains::is_safe_command(command)
}

#[test]
fn storing_a_variable_that_changes_later_shells_asks() {
    for name in [
        "PROMPT_COMMAND", "PS0", "PS1", "PS4", "RPROMPT", "FPATH", "ZDOTDIR", "ENV", "BASH_ENV", "PATH", "CDPATH", "SUDO_ASKPASS",
        "GIT_ASKPASS", "http_proxy", "HTTPS_PROXY", "all_proxy", "NO_PROXY", "SSL_CERT_FILE", "SSL_CERT_DIR", "REQUESTS_CA_BUNDLE",
        "NODE_EXTRA_CA_CERTS", "CURL_CA_BUNDLE", "GIT_SSL_CAINFO", "npm_config_registry", "NPM_CONFIG_REGISTRY", "PIP_INDEX_URL",
        "PIP_EXTRA_INDEX_URL", "UV_INDEX_URL", "CARGO_REGISTRIES_CRATES_IO_INDEX", "RUBYGEMS_HOST", "COMPOSER_HOME",
    ] {
        for value in ["x", "./bin", "https://example.test/", ""] {
            for line in [
                format!("mise set {name}={value}"),
                format!("mise set --file ./mise.toml {name}={value}"),
                format!("mise set -E staging {name}={value}"),
            ] {
                assert!(!approved(&line), "`{line}` was approved");
            }
        }
    }
}

#[test]
fn storing_an_ordinary_variable_stays_approved() {
    for line in ["mise set FOO=bar", "mise set NODE_ENV=production", "mise set -E staging RUST_LOG=debug", "mise unset PATH"] {
        assert!(approved(line), "`{line}` should stay approved");
    }
}
