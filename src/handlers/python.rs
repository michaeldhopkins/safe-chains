use crate::parse::{Segment, Token, WordSet, has_flag};
use crate::policy::{self, FlagPolicy};

static PIP_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--editable", "--exclude-editable", "--include-editable",
        "--local", "--not-required", "--outdated", "--pre",
        "--uptodate", "--user",
    ]),
    standalone_short: b"eilo",
    valued: WordSet::new(&[
        "--exclude", "--format", "--index-url", "--path",
    ]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static PIP_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--files", "--verbose"]),
    standalone_short: b"fv",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static PIP_FREEZE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--exclude-editable", "--local", "--user",
    ]),
    standalone_short: b"l",
    valued: WordSet::new(&["--exclude", "--path"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static PIP_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_pip(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    match tokens[1].as_str() {
        "list" => policy::check(&tokens[1..], &PIP_LIST_POLICY),
        "show" => policy::check(&tokens[1..], &PIP_SHOW_POLICY),
        "freeze" => policy::check(&tokens[1..], &PIP_FREEZE_POLICY),
        "check" | "debug" | "help" | "index" | "inspect" => {
            policy::check(&tokens[1..], &PIP_BARE_POLICY)
        }
        "config" => tokens
            .get(2)
            .is_some_and(|a| a == "list" || a == "get"),
        _ => false,
    }
}

static UV_PIP_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--editable", "--exclude-editable", "--outdated",
        "--strict",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&["--exclude", "--format", "--python"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static UV_PIP_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--files", "--verbose"]),
    standalone_short: b"v",
    valued: WordSet::new(&["--python"]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static UV_PIP_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--verbose"]),
    standalone_short: b"v",
    valued: WordSet::new(&["--python"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static UV_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--verbose"]),
    standalone_short: b"v",
    valued: WordSet::new(&["--python"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_uv(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if tokens[1] == "pip" {
        if tokens.len() < 3 {
            return false;
        }
        let policy = match tokens[2].as_str() {
            "list" => &UV_PIP_LIST_POLICY,
            "show" => &UV_PIP_SHOW_POLICY,
            "check" | "freeze" => &UV_PIP_SIMPLE_POLICY,
            _ => return false,
        };
        return policy::check(&tokens[2..], policy);
    }
    if tokens[1] == "python" {
        return tokens.get(2).is_some_and(|a| a == "list")
            && policy::check(&tokens[2..], &UV_SIMPLE_POLICY);
    }
    if tokens[1] == "tool" {
        return tokens.get(2).is_some_and(|a| a == "list")
            && policy::check(&tokens[2..], &UV_SIMPLE_POLICY);
    }
    false
}

static POETRY_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--latest", "--no-dev", "--outdated",
        "--top-level", "--tree",
    ]),
    standalone_short: b"loT",
    valued: WordSet::new(&["--why"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static POETRY_CHECK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--lock"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static POETRY_ENV_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--full-path"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_poetry(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if tokens[1] == "show" {
        return policy::check(&tokens[1..], &POETRY_SHOW_POLICY);
    }
    if tokens[1] == "check" {
        return policy::check(&tokens[1..], &POETRY_CHECK_POLICY);
    }
    if tokens[1] == "env" {
        return tokens.get(2).is_some_and(|a| a == "info" || a == "list")
            && policy::check(&tokens[2..], &POETRY_ENV_POLICY);
    }
    false
}

static PYENV_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--bare"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_pyenv(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    static PYENV_SAFE: WordSet = WordSet::new(&[
        "help", "root", "shims", "version", "versions", "which",
    ]);
    if !PYENV_SAFE.contains(&tokens[1]) {
        return false;
    }
    policy::check(&tokens[1..], &PYENV_BARE_POLICY)
}

static CONDA_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--explicit", "--export", "--full-name", "--json",
        "--no-pip", "--revisions",
    ]),
    standalone_short: b"ef",
    valued: WordSet::new(&["--name", "--prefix"]),
    valued_short: b"np",
    bare: true,
    max_positional: None,
};

static CONDA_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--envs", "--json", "--verbose",
    ]),
    standalone_short: b"aev",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static CONDA_CONFIG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--json", "--quiet", "--show", "--show-sources", "--verbose",
    ]),
    standalone_short: b"qv",
    valued: WordSet::new(&["--env", "--file", "--name", "--prefix"]),
    valued_short: b"fnp",
    bare: false,
    max_positional: None,
};

pub fn is_safe_conda(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    match tokens[1].as_str() {
        "list" => policy::check(&tokens[1..], &CONDA_LIST_POLICY),
        "info" => policy::check(&tokens[1..], &CONDA_INFO_POLICY),
        "config" => (has_flag(&tokens[1..], None, Some("--show"))
                || has_flag(&tokens[1..], None, Some("--show-sources")))
            && policy::check(&tokens[1..], &CONDA_CONFIG_POLICY),
        _ => false,
    }
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "pip" | "pip3" => Some(is_safe_pip(tokens)),
        "uv" => Some(is_safe_uv(tokens)),
        "poetry" => Some(is_safe_poetry(tokens)),
        "pyenv" => Some(is_safe_pyenv(tokens)),
        "conda" => Some(is_safe_conda(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("pip / pip3",
            "Subcommands: check, config (list/get), debug, freeze, help, index, inspect, \
             list, show."),
        CommandDoc::handler("uv",
            "Subcommands: pip check/freeze/list/show, python list, tool list. \
            "),
        CommandDoc::handler("poetry",
            "Subcommands: check, env info/list, show."),
        CommandDoc::handler("pyenv",
            "Subcommands: help, root, shims, version, versions, which. \
             Minimal flags allowed (--bare)."),
        CommandDoc::handler("conda",
            "Subcommands: config (--show/--show-sources only), info, list. \
            "),
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
        pip_list_outdated: "pip list --outdated",
        pip_list_format: "pip list --format json",
        pip_show: "pip show requests",
        pip_show_files: "pip show requests --files",
        pip_freeze: "pip freeze",
        pip_freeze_all: "pip freeze --all",
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
        uv_pip_list_outdated: "uv pip list --outdated",
        uv_pip_show: "uv pip show requests",
        uv_pip_show_files: "uv pip show requests --files",
        uv_pip_freeze: "uv pip freeze",
        uv_pip_check: "uv pip check",
        uv_tool_list: "uv tool list",
        uv_python_list: "uv python list",
        poetry_show: "poetry show",
        poetry_show_tree: "poetry show --tree",
        poetry_show_outdated: "poetry show --outdated",
        poetry_show_latest: "poetry show --latest",
        poetry_check: "poetry check",
        poetry_check_lock: "poetry check --lock",
        poetry_version: "poetry --version",
        poetry_env_info: "poetry env info",
        poetry_env_info_full: "poetry env info --full-path",
        poetry_env_list: "poetry env list",
        pyenv_versions: "pyenv versions",
        pyenv_versions_bare: "pyenv versions --bare",
        pyenv_version: "pyenv version",
        pyenv_which: "pyenv which python",
        pyenv_root: "pyenv root",
        pyenv_shims: "pyenv shims",
        pyenv_version_flag: "pyenv --version",
        pyenv_help: "pyenv help",
        conda_list: "conda list",
        conda_list_json: "conda list --json",
        conda_list_export: "conda list --export",
        conda_list_name: "conda list --name myenv",
        conda_info: "conda info",
        conda_info_envs: "conda info --envs",
        conda_info_json: "conda info --json",
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
        pip_list_unknown_denied: "pip list --unknown",
        pip_show_unknown_denied: "pip show requests --unknown",
        uv_pip_install_denied: "uv pip install requests",
        uv_run_denied: "uv run script.py",
        uv_venv_denied: "uv venv",
        uv_add_denied: "uv add requests",
        bare_uv_denied: "uv",
        uv_pip_list_unknown_denied: "uv pip list --unknown",
        poetry_install_denied: "poetry install",
        poetry_add_denied: "poetry add requests",
        poetry_build_denied: "poetry build",
        poetry_show_unknown_denied: "poetry show --unknown",
        pyenv_install_denied: "pyenv install 3.12",
        pyenv_global_denied: "pyenv global 3.12",
        pyenv_local_denied: "pyenv local 3.12",
        pyenv_versions_unknown_denied: "pyenv versions --unknown",
        conda_install_denied: "conda install numpy",
        conda_create_denied: "conda create -n myenv",
        conda_remove_denied: "conda remove numpy",
        conda_config_show_with_set_denied: "conda config --show --set always_yes true",
        conda_config_show_sources_with_remove_denied: "conda config --show-sources --remove channels defaults",
        conda_config_set_denied: "conda config --set always_yes true",
        conda_config_add_denied: "conda config --add channels conda-forge",
        conda_list_unknown_denied: "conda list --unknown",
        conda_info_unknown_denied: "conda info --unknown",
    }
}
