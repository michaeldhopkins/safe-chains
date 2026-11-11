//! `mise set -g` writes into the global config, whose `[env]` `mise activate` exports in every
//! folder. A value judged safe for this project (`BASH_ENV=./x.sh`, `PROMPT_COMMAND='cargo build'`)
//! then resolves against whatever folder a later shell is in, so no variable `envvars.toml` audits
//! may be set globally, whatever its value. An unlisted name (`FOO=1`) is unaffected.
use crate::parse::Token;

pub(crate) fn sets_an_audited_variable_globally(tokens: &[Token]) -> bool {
    if tokens.get(1).is_none_or(|t| *t != "set") {
        return false;
    }
    let options = tokens[2..].iter().take_while(|t| **t != "--");
    let global = options
        .clone()
        .any(|t| *t == "--global" || (t.starts_with('-') && !t.starts_with("--") && t.contains('g')));
    global
        && tokens[2..]
            .iter()
            .filter_map(|t| t.as_str().split_once('='))
            .any(|(name, _)| crate::envvars::shape_of(name).is_some())
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    #[test]
    fn an_audited_variable_is_never_set_globally() {
        for name in [
            "LD_PRELOAD=./x.so", "DYLD_INSERT_LIBRARIES=./x.dylib", "BASH_ENV=./x.sh", "RUSTC_WRAPPER=./w", "GIT_PAGER=./p",
            "PYTHONPATH=.", "PROMPT_COMMAND='cargo build'", "PROMPT_COMMAND=./x.sh", "PROMPT_COMMAND=id",
        ] {
            for prefix in ["mise set -g", "mise set --global", "mise set -E staging -g", "mise set -g FOO=1"] {
                let cmd = format!("{prefix} {name}");
                assert!(!is_safe_command(&cmd), "{cmd} must be refused: the global config applies in every folder");
            }
        }
    }

    #[test]
    fn a_plain_variable_is_set_globally_and_an_audited_one_locally() {
        for cmd in [
            "mise set -g FOO=1", "mise set --global FOO=bar", "mise set -E staging -g FOO=1", "mise set BASH_ENV=./x.sh",
            "mise set RUSTC_WRAPPER=./w",
        ] {
            assert!(is_safe_command(cmd), "{cmd} should be approved");
        }
    }

    proptest::proptest! {
        #[test]
        fn no_audited_variable_is_set_globally_in_any_spelling(
            name in proptest::sample::select(vec!["LD_PRELOAD", "BASH_ENV", "RUSTC_WRAPPER", "GIT_PAGER", "PYTHONPATH", "PROMPT_COMMAND", "EDITOR"]),
            value in "[a-z./]{0,8}",
            flag in proptest::sample::select(vec!["-g", "--global"]),
            before in proptest::bool::ANY,
        ) {
            let cmd = if before { format!("mise set {flag} {name}={value}") } else { format!("mise set {name}={value} {flag}") };
            proptest::prop_assert!(!is_safe_command(&cmd), "{}", cmd);
        }
    }
}
