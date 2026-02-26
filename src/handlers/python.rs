use crate::parse::{FlagCheck, Token, WordSet};

static PIP_READ_ONLY: WordSet = WordSet::new(&[
    "--version", "check", "debug", "freeze", "help", "index", "inspect", "list", "show",
]);

static UV_SAFE: WordSet =
    WordSet::new(&["--version"]);

static UV_MULTI: &[(&str, WordSet)] = &[
    ("pip", WordSet::new(&["check", "freeze", "list", "show"])),
    ("python", WordSet::new(&["list"])),
    ("tool", WordSet::new(&["list"])),
];

static POETRY_SAFE: WordSet =
    WordSet::new(&["--version", "check", "show"]);

static POETRY_MULTI: &[(&str, WordSet)] =
    &[("env", WordSet::new(&["info", "list"]))];

static PYENV_SAFE: WordSet = WordSet::new(&[
    "--version", "help", "root", "shims", "version", "versions", "which",
]);

static CONDA_SAFE: WordSet =
    WordSet::new(&["--version", "info", "list"]);

static CONDA_CONFIG: FlagCheck = FlagCheck::new(
    &["--show", "--show-sources"],
    &["--add", "--append", "--prepend", "--remove", "--remove-key", "--set"],
);

pub fn is_safe_pip(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if PIP_READ_ONLY.contains(&tokens[1]) {
        return true;
    }
    if tokens[1] == "config" {
        return tokens
            .get(2)
            .is_some_and(|a| a == "list" || a == "get");
    }
    false
}

pub fn is_safe_uv(tokens: &[Token]) -> bool {
    super::is_safe_subcmd(tokens, &UV_SAFE, UV_MULTI)
}

pub fn is_safe_poetry(tokens: &[Token]) -> bool {
    super::is_safe_subcmd(tokens, &POETRY_SAFE, POETRY_MULTI)
}

pub fn is_safe_pyenv(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && PYENV_SAFE.contains(&tokens[1])
}

pub fn is_safe_conda(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if CONDA_SAFE.contains(&tokens[1]) {
        return true;
    }
    if tokens[1] == "config" {
        return CONDA_CONFIG.is_safe(&tokens[2..]);
    }
    false
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, describe_wordset};
    vec![
        CommandDoc::handler("pip / pip3", format!(
            "{} Guarded: config (list/get only).",
            describe_wordset(&PIP_READ_ONLY),
        )),
        CommandDoc::wordset_multi("uv", &UV_SAFE, UV_MULTI),
        CommandDoc::wordset_multi("poetry", &POETRY_SAFE, POETRY_MULTI),
        CommandDoc::wordset("pyenv", &PYENV_SAFE),
        CommandDoc::handler("conda", format!(
            "{} Guarded: config ({} only).",
            describe_wordset(&CONDA_SAFE),
            CONDA_CONFIG.required().iter().collect::<Vec<_>>().join(", "),
        )),
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    #[test]
    fn pip_list() {
        assert!(check("pip list"));
    }

    #[test]
    fn pip_show() {
        assert!(check("pip show requests"));
    }

    #[test]
    fn pip_freeze() {
        assert!(check("pip freeze"));
    }

    #[test]
    fn pip_check() {
        assert!(check("pip check"));
    }

    #[test]
    fn pip_index() {
        assert!(check("pip index versions requests"));
    }

    #[test]
    fn pip_debug() {
        assert!(check("pip debug"));
    }

    #[test]
    fn pip_inspect() {
        assert!(check("pip inspect"));
    }

    #[test]
    fn pip_help() {
        assert!(check("pip help"));
    }

    #[test]
    fn pip_config_list() {
        assert!(check("pip config list"));
    }

    #[test]
    fn pip_config_get() {
        assert!(check("pip config get global.index-url"));
    }

    #[test]
    fn pip3_list() {
        assert!(check("pip3 list"));
    }

    #[test]
    fn pip3_show() {
        assert!(check("pip3 show flask"));
    }

    #[test]
    fn pip3_freeze() {
        assert!(check("pip3 freeze"));
    }

    #[test]
    fn pip_version() {
        assert!(check("pip --version"));
    }

    #[test]
    fn pip3_version() {
        assert!(check("pip3 --version"));
    }

    #[test]
    fn pip_install_denied() {
        assert!(!check("pip install requests"));
    }

    #[test]
    fn pip_uninstall_denied() {
        assert!(!check("pip uninstall flask"));
    }

    #[test]
    fn pip3_install_denied() {
        assert!(!check("pip3 install django"));
    }

    #[test]
    fn bare_pip_denied() {
        assert!(!check("pip"));
    }

    #[test]
    fn pip_config_set_denied() {
        assert!(!check("pip config set global.index-url https://example.com"));
    }

    #[test]
    fn uv_version() {
        assert!(check("uv --version"));
    }

    #[test]
    fn uv_pip_list() {
        assert!(check("uv pip list"));
    }

    #[test]
    fn uv_pip_show() {
        assert!(check("uv pip show requests"));
    }

    #[test]
    fn uv_pip_freeze() {
        assert!(check("uv pip freeze"));
    }

    #[test]
    fn uv_pip_check() {
        assert!(check("uv pip check"));
    }

    #[test]
    fn uv_tool_list() {
        assert!(check("uv tool list"));
    }

    #[test]
    fn uv_python_list() {
        assert!(check("uv python list"));
    }

    #[test]
    fn uv_pip_install_denied() {
        assert!(!check("uv pip install requests"));
    }

    #[test]
    fn uv_run_denied() {
        assert!(!check("uv run script.py"));
    }

    #[test]
    fn uv_venv_denied() {
        assert!(!check("uv venv"));
    }

    #[test]
    fn uv_add_denied() {
        assert!(!check("uv add requests"));
    }

    #[test]
    fn bare_uv_denied() {
        assert!(!check("uv"));
    }

    #[test]
    fn poetry_show() {
        assert!(check("poetry show"));
    }

    #[test]
    fn poetry_check() {
        assert!(check("poetry check"));
    }

    #[test]
    fn poetry_version() {
        assert!(check("poetry --version"));
    }

    #[test]
    fn poetry_env_info() {
        assert!(check("poetry env info"));
    }

    #[test]
    fn poetry_env_list() {
        assert!(check("poetry env list"));
    }

    #[test]
    fn poetry_install_denied() {
        assert!(!check("poetry install"));
    }

    #[test]
    fn poetry_add_denied() {
        assert!(!check("poetry add requests"));
    }

    #[test]
    fn poetry_build_denied() {
        assert!(!check("poetry build"));
    }

    #[test]
    fn pyenv_versions() {
        assert!(check("pyenv versions"));
    }

    #[test]
    fn pyenv_version() {
        assert!(check("pyenv version"));
    }

    #[test]
    fn pyenv_which() {
        assert!(check("pyenv which python"));
    }

    #[test]
    fn pyenv_root() {
        assert!(check("pyenv root"));
    }

    #[test]
    fn pyenv_shims() {
        assert!(check("pyenv shims"));
    }

    #[test]
    fn pyenv_version_flag() {
        assert!(check("pyenv --version"));
    }

    #[test]
    fn pyenv_help() {
        assert!(check("pyenv help"));
    }

    #[test]
    fn pyenv_install_denied() {
        assert!(!check("pyenv install 3.12"));
    }

    #[test]
    fn pyenv_global_denied() {
        assert!(!check("pyenv global 3.12"));
    }

    #[test]
    fn pyenv_local_denied() {
        assert!(!check("pyenv local 3.12"));
    }

    #[test]
    fn conda_list() {
        assert!(check("conda list"));
    }

    #[test]
    fn conda_info() {
        assert!(check("conda info"));
    }

    #[test]
    fn conda_version() {
        assert!(check("conda --version"));
    }

    #[test]
    fn conda_config_show() {
        assert!(check("conda config --show"));
    }

    #[test]
    fn conda_config_show_sources() {
        assert!(check("conda config --show-sources"));
    }

    #[test]
    fn conda_install_denied() {
        assert!(!check("conda install numpy"));
    }

    #[test]
    fn conda_create_denied() {
        assert!(!check("conda create -n myenv"));
    }

    #[test]
    fn conda_remove_denied() {
        assert!(!check("conda remove numpy"));
    }

    #[test]
    fn conda_config_show_with_set_denied() {
        assert!(!check("conda config --show --set always_yes true"));
    }

    #[test]
    fn conda_config_show_sources_with_remove_denied() {
        assert!(!check("conda config --show-sources --remove channels defaults"));
    }

    #[test]
    fn conda_config_set_denied() {
        assert!(!check("conda config --set always_yes true"));
    }

    #[test]
    fn conda_config_add_denied() {
        assert!(!check("conda config --add channels conda-forge"));
    }
}
