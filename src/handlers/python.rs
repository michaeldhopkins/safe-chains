use crate::command::{CheckFn, CommandDef, SubDef};
use crate::parse::{Segment, Token, WordSet, has_flag};
use crate::policy::{self, FlagPolicy, FlagStyle};

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
    flag_style: FlagStyle::Strict,
};

static PIP_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--files", "--verbose"]),
    standalone_short: b"fv",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
};

static PIP_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static PIP_SUBS: &[SubDef] = &[
    SubDef::Policy { name: "list", policy: &PIP_LIST_POLICY },
    SubDef::Policy { name: "show", policy: &PIP_SHOW_POLICY },
    SubDef::Policy { name: "freeze", policy: &PIP_FREEZE_POLICY },
    SubDef::Policy { name: "check", policy: &PIP_BARE_POLICY },
    SubDef::Nested { name: "config", subs: &[
        SubDef::Policy { name: "get", policy: &PIP_BARE_POLICY },
        SubDef::Policy { name: "list", policy: &PIP_BARE_POLICY },
    ]},
    SubDef::Policy { name: "debug", policy: &PIP_BARE_POLICY },
    SubDef::Policy { name: "help", policy: &PIP_BARE_POLICY },
    SubDef::Policy { name: "index", policy: &PIP_BARE_POLICY },
    SubDef::Policy { name: "inspect", policy: &PIP_BARE_POLICY },
];

pub(crate) static PIP: CommandDef = CommandDef {
    name: "pip",
    subs: PIP_SUBS,
    bare_flags: &[],
    help_eligible: true,
};

pub(crate) static PIP3: CommandDef = CommandDef {
    name: "pip3",
    subs: PIP_SUBS,
    bare_flags: &[],
    help_eligible: true,
};

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
    flag_style: FlagStyle::Strict,
};

static UV_PIP_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--files", "--verbose"]),
    standalone_short: b"v",
    valued: WordSet::new(&["--python"]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static UV_PIP_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--verbose"]),
    standalone_short: b"v",
    valued: WordSet::new(&["--python"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static UV_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--verbose"]),
    standalone_short: b"v",
    valued: WordSet::new(&["--python"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static UV: CommandDef = CommandDef {
    name: "uv",
    subs: &[
        SubDef::Nested { name: "pip", subs: &[
            SubDef::Policy { name: "list", policy: &UV_PIP_LIST_POLICY },
            SubDef::Policy { name: "show", policy: &UV_PIP_SHOW_POLICY },
            SubDef::Policy { name: "check", policy: &UV_PIP_SIMPLE_POLICY },
            SubDef::Policy { name: "freeze", policy: &UV_PIP_SIMPLE_POLICY },
        ]},
        SubDef::Nested { name: "python", subs: &[
            SubDef::Policy { name: "list", policy: &UV_SIMPLE_POLICY },
        ]},
        SubDef::Nested { name: "tool", subs: &[
            SubDef::Policy { name: "list", policy: &UV_SIMPLE_POLICY },
        ]},
    ],
    bare_flags: &[],
    help_eligible: true,
};

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
    flag_style: FlagStyle::Strict,
};

static POETRY_CHECK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--lock"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static POETRY_ENV_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--full-path"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static POETRY: CommandDef = CommandDef {
    name: "poetry",
    subs: &[
        SubDef::Policy { name: "show", policy: &POETRY_SHOW_POLICY },
        SubDef::Policy { name: "check", policy: &POETRY_CHECK_POLICY },
        SubDef::Nested { name: "env", subs: &[
            SubDef::Policy { name: "info", policy: &POETRY_ENV_POLICY },
            SubDef::Policy { name: "list", policy: &POETRY_ENV_POLICY },
        ]},
    ],
    bare_flags: &[],
    help_eligible: true,
};

static PYENV_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--bare"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static PYENV: CommandDef = CommandDef {
    name: "pyenv",
    subs: &[
        SubDef::Policy { name: "help", policy: &PYENV_BARE_POLICY },
        SubDef::Policy { name: "root", policy: &PYENV_BARE_POLICY },
        SubDef::Policy { name: "shims", policy: &PYENV_BARE_POLICY },
        SubDef::Policy { name: "version", policy: &PYENV_BARE_POLICY },
        SubDef::Policy { name: "versions", policy: &PYENV_BARE_POLICY },
        SubDef::Policy { name: "which", policy: &PYENV_BARE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
};

fn check_conda_config(tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    (has_flag(tokens, None, Some("--show"))
        || has_flag(tokens, None, Some("--show-sources")))
        && policy::check(tokens, &CONDA_CONFIG_POLICY)
}

pub(crate) static CONDA: CommandDef = CommandDef {
    name: "conda",
    subs: &[
        SubDef::Policy { name: "list", policy: &CONDA_LIST_POLICY },
        SubDef::Policy { name: "info", policy: &CONDA_INFO_POLICY },
        SubDef::Custom { name: "config", check: check_conda_config as CheckFn, doc: "config (--show/--show-sources only).", test_suffix: Some("--show") },
    ],
    bare_flags: &[],
    help_eligible: true,
};

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    PIP.dispatch(cmd, tokens, is_safe)
        .or_else(|| PIP3.dispatch(cmd, tokens, is_safe))
        .or_else(|| UV.dispatch(cmd, tokens, is_safe))
        .or_else(|| POETRY.dispatch(cmd, tokens, is_safe))
        .or_else(|| PYENV.dispatch(cmd, tokens, is_safe))
        .or_else(|| CONDA.dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut pip_doc = PIP.to_doc();
    pip_doc.name = "pip / pip3";
    vec![pip_doc, UV.to_doc(), POETRY.to_doc(), PYENV.to_doc(), CONDA.to_doc()]
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
        pip_config_set_denied: "pip config set global.index-url https://example.com",
        conda_config_show_with_set_denied: "conda config --show --set always_yes true",
        conda_config_show_sources_with_remove_denied: "conda config --show-sources --remove channels defaults",
        conda_config_set_denied: "conda config --set always_yes true",
        conda_config_add_denied: "conda config --add channels conda-forge",
    }
}
