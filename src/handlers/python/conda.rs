use crate::command::{CheckFn, CommandDef, SubDef};
use crate::parse::{Segment, Token, WordSet, has_flag};
use crate::policy::{self, FlagPolicy, FlagStyle};

static CONDA_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--explicit", "--export", "--full-name", "--json",
        "--no-pip", "--revisions",
    ]),
    standalone_short: b"ef",
    valued: WordSet::flags(&["--name", "--prefix"]),
    valued_short: b"np",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static CONDA_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--envs", "--json", "--verbose",
    ]),
    standalone_short: b"aev",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static CONDA_CONFIG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--json", "--quiet", "--show", "--show-sources", "--verbose",
    ]),
    standalone_short: b"qv",
    valued: WordSet::flags(&["--env", "--file", "--name", "--prefix"]),
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
    url: "https://docs.conda.io/projects/conda/en/stable/commands/index.html",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
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
        conda_config_show_with_set_denied: "conda config --show --set always_yes true",
        conda_config_show_sources_with_remove_denied: "conda config --show-sources --remove channels defaults",
        conda_config_set_denied: "conda config --set always_yes true",
        conda_config_add_denied: "conda config --add channels conda-forge",
    }
}
