use crate::verdict::{SafetyLevel, Verdict};
use crate::command::{CheckFn, CommandDef, SubDef};
use crate::parse::{Token, WordSet, has_flag};
use crate::policy::{self, FlagPolicy, FlagStyle};

static CONDA_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--explicit", "--export", "--full-name", "--help", "--json",
        "--no-pip", "--revisions",
        "-e", "-f", "-h",
    ]),
    valued: WordSet::flags(&["--name", "--prefix", "-n", "-p"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static CONDA_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--envs", "--help", "--json", "--verbose",
        "-a", "-e", "-h", "-v",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static CONDA_CONFIG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--json", "--quiet", "--show", "--show-sources", "--verbose",
        "-h", "-q", "-v",
    ]),
    valued: WordSet::flags(&["--env", "--file", "--name", "--prefix", "-f", "-n", "-p"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn check_conda_config(tokens: &[Token]) -> Verdict {
    if (has_flag(tokens, None, Some("--show"))
        || has_flag(tokens, None, Some("--show-sources")))
        && policy::check(tokens, &CONDA_CONFIG_POLICY)
    { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }
}

pub(crate) static CONDA: CommandDef = CommandDef {
    name: "conda",
    subs: &[
        SubDef::Policy { name: "list", policy: &CONDA_LIST_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "info", policy: &CONDA_INFO_POLICY, level: SafetyLevel::Inert },
        SubDef::Custom { name: "config", check: check_conda_config as CheckFn, doc: "config (--show/--show-sources only).", test_suffix: Some("--show") },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://docs.conda.io/projects/conda/en/stable/commands/index.html",
    aliases: &[],
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
