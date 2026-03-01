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
    use crate::docs::{CommandDoc, doc};
    vec![
        CommandDoc::handler("pip / pip3",
            doc(&PIP_READ_ONLY)
                .section("Guarded: config (list/get only).")
                .build()),
        CommandDoc::wordset_multi("uv", &UV_SAFE, UV_MULTI),
        CommandDoc::wordset_multi("poetry", &POETRY_SAFE, POETRY_MULTI),
        CommandDoc::wordset("pyenv", &PYENV_SAFE),
        CommandDoc::handler("conda",
            doc(&CONDA_SAFE)
                .section(format!("Guarded: config ({} only).",
                    CONDA_CONFIG.required().iter().collect::<Vec<_>>().join(", ")))
                .build()),
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        pip_list: "pip list",
        pip_show: "pip show requests",
        pip_freeze: "pip freeze",
        pip_check: "pip check",
        pip_index: "pip index versions requests",
        pip_debug: "pip debug",
        pip_inspect: "pip inspect",
        pip_help: "pip help",
        pip_config_list: "pip config list",
        pip_config_get: "pip config get global.index-url",
        pip3_list: "pip3 list",
        pip3_show: "pip3 show flask",
        pip3_freeze: "pip3 freeze",
        pip_version: "pip --version",
        pip3_version: "pip3 --version",
        uv_version: "uv --version",
        uv_pip_list: "uv pip list",
        uv_pip_show: "uv pip show requests",
        uv_pip_freeze: "uv pip freeze",
        uv_pip_check: "uv pip check",
        uv_tool_list: "uv tool list",
        uv_python_list: "uv python list",
        poetry_show: "poetry show",
        poetry_check: "poetry check",
        poetry_version: "poetry --version",
        poetry_env_info: "poetry env info",
        poetry_env_list: "poetry env list",
        pyenv_versions: "pyenv versions",
        pyenv_version: "pyenv version",
        pyenv_which: "pyenv which python",
        pyenv_root: "pyenv root",
        pyenv_shims: "pyenv shims",
        pyenv_version_flag: "pyenv --version",
        pyenv_help: "pyenv help",
        conda_list: "conda list",
        conda_info: "conda info",
        conda_version: "conda --version",
        conda_config_show: "conda config --show",
        conda_config_show_sources: "conda config --show-sources",
    }

    denied! {
        pip_install_denied: "pip install requests",
        pip_uninstall_denied: "pip uninstall flask",
        pip3_install_denied: "pip3 install django",
        bare_pip_denied: "pip",
        pip_config_set_denied: "pip config set global.index-url https://example.com",
        uv_pip_install_denied: "uv pip install requests",
        uv_run_denied: "uv run script.py",
        uv_venv_denied: "uv venv",
        uv_add_denied: "uv add requests",
        bare_uv_denied: "uv",
        poetry_install_denied: "poetry install",
        poetry_add_denied: "poetry add requests",
        poetry_build_denied: "poetry build",
        pyenv_install_denied: "pyenv install 3.12",
        pyenv_global_denied: "pyenv global 3.12",
        pyenv_local_denied: "pyenv local 3.12",
        conda_install_denied: "conda install numpy",
        conda_create_denied: "conda create -n myenv",
        conda_remove_denied: "conda remove numpy",
        conda_config_show_with_set_denied: "conda config --show --set always_yes true",
        conda_config_show_sources_with_remove_denied: "conda config --show-sources --remove channels defaults",
        conda_config_set_denied: "conda config --set always_yes true",
        conda_config_add_denied: "conda config --add channels conda-forge",
    }
}
