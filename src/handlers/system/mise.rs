use crate::command::{CheckFn, CommandDef, SubDef};
use crate::verdict::{SafetyLevel, Verdict};
use crate::parse::{Token, WordSet};
use crate::policy::{FlagPolicy, FlagStyle};

static HELP_ONLY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static MISE_RESHIM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--force", "--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static MISE_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h", "-q", "-v"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static MISE_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--current", "--help", "--installed", "--json", "--missing",
        "--no-header", "--prefix",
        "-J", "-c", "-h", "-i", "-m",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static MISE_ENV_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--json", "-J", "-h"]),
    valued: WordSet::flags(&["--shell", "-s"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static MISE_OUTDATED_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--json", "--no-header", "-J", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static MISE_SEARCH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static MISE_REGISTRY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&["--backend", "-b"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static MISE_LS_REMOTE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--all", "--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static MISE_TRUST_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--show", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static MISE_FMT_CHECK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--check", "--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static MISE_SET_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--json", "-J", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static SETTINGS_SUBS: &[SubDef] = &[
    SubDef::Policy { name: "get", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "list", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "ls", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "add", policy: &HELP_ONLY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "set", policy: &HELP_ONLY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "unset", policy: &HELP_ONLY, level: SafetyLevel::Inert },
];

static CONFIG_SUBS: &[SubDef] = &[
    SubDef::Policy { name: "get", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "list", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "ls", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "generate", policy: &HELP_ONLY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "set", policy: &HELP_ONLY, level: SafetyLevel::Inert },
];

static PLUGINS_SUBS: &[SubDef] = &[
    SubDef::Policy { name: "list", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "ls", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "ls-remote", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "install", policy: &HELP_ONLY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "link", policy: &HELP_ONLY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "uninstall", policy: &HELP_ONLY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "update", policy: &HELP_ONLY, level: SafetyLevel::Inert },
];

static BACKENDS_SUBS: &[SubDef] = &[
    SubDef::Policy { name: "list", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "ls", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
];

static TASKS_SUBS: &[SubDef] = &[
    SubDef::Policy { name: "deps", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "info", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "list", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "ls", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "add", policy: &HELP_ONLY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "edit", policy: &HELP_ONLY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "run", policy: &HELP_ONLY, level: SafetyLevel::Inert },
];

static CACHE_SUBS: &[SubDef] = &[
    SubDef::Policy { name: "path", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "clear", policy: &HELP_ONLY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "prune", policy: &HELP_ONLY, level: SafetyLevel::Inert },
];

static TOOL_ALIAS_SUBS: &[SubDef] = &[
    SubDef::Policy { name: "get", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "list", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "ls", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "set", policy: &HELP_ONLY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "unset", policy: &HELP_ONLY, level: SafetyLevel::Inert },
];

static SHELL_ALIAS_SUBS: &[SubDef] = &[
    SubDef::Policy { name: "get", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "list", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "ls", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "set", policy: &HELP_ONLY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "unset", policy: &HELP_ONLY, level: SafetyLevel::Inert },
];

fn check_nested_bare(tokens: &[Token], subs: &[SubDef]) -> Verdict {
    if tokens.len() == 1 {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let sub = tokens[1].as_str();
    if tokens.len() == 2 && (sub == "--help" || sub == "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    subs.iter()
        .find(|s| s.name() == sub)
        .map(|s| s.check(&tokens[1..]))
        .unwrap_or(Verdict::Denied)
}

fn check_mise_settings(tokens: &[Token]) -> Verdict {
    check_nested_bare(tokens, SETTINGS_SUBS)
}

fn check_mise_config(tokens: &[Token]) -> Verdict {
    check_nested_bare(tokens, CONFIG_SUBS)
}

fn check_mise_plugins(tokens: &[Token]) -> Verdict {
    check_nested_bare(tokens, PLUGINS_SUBS)
}

fn check_mise_backends(tokens: &[Token]) -> Verdict {
    check_nested_bare(tokens, BACKENDS_SUBS)
}

fn check_mise_tasks(tokens: &[Token]) -> Verdict {
    check_nested_bare(tokens, TASKS_SUBS)
}

fn check_mise_cache(tokens: &[Token]) -> Verdict {
    check_nested_bare(tokens, CACHE_SUBS)
}

fn check_mise_tool_alias(tokens: &[Token]) -> Verdict {
    check_nested_bare(tokens, TOOL_ALIAS_SUBS)
}

fn check_mise_shell_alias(tokens: &[Token]) -> Verdict {
    check_nested_bare(tokens, SHELL_ALIAS_SUBS)
}

fn check_mise_exec(tokens: &[Token]) -> Verdict {
    let sep = tokens[1..].iter().position(|t| *t == "--");
    if let Some(pos) = sep {
        let inner_start = 1 + pos + 1;
        if inner_start >= tokens.len() {
            return Verdict::Denied;
        }
        let inner = shell_words::join(tokens[inner_start..].iter().map(|t| t.as_str()));
        return crate::command_verdict(&inner);
    }
    Verdict::Denied

}

pub(crate) static MISE: CommandDef = CommandDef {
    name: "mise",
    subs: &[
        SubDef::Policy { name: "list", policy: &MISE_LIST_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "ls", policy: &MISE_LIST_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "current", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "which", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "where", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "doctor", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "reshim", policy: &MISE_RESHIM_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "env", policy: &MISE_ENV_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "outdated", policy: &MISE_OUTDATED_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "search", policy: &MISE_SEARCH_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "registry", policy: &MISE_REGISTRY_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "latest", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "ls-remote", policy: &MISE_LS_REMOTE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "bin-paths", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "tool", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "completion", policy: &MISE_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "trust", policy: &MISE_TRUST_SHOW_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "fmt", policy: &MISE_FMT_CHECK_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "set", policy: &MISE_SET_POLICY, level: SafetyLevel::Inert },
        SubDef::Custom { name: "config", check: check_mise_config as CheckFn, doc: "Bare invocation allowed. Subcommands: get, list, ls.", test_suffix: None },
        SubDef::Custom { name: "settings", check: check_mise_settings as CheckFn, doc: "Bare invocation allowed. Subcommands: get, list, ls.", test_suffix: None },
        SubDef::Custom { name: "plugins", check: check_mise_plugins as CheckFn, doc: "Bare invocation allowed. Subcommands: list, ls, ls-remote.", test_suffix: None },
        SubDef::Custom { name: "backends", check: check_mise_backends as CheckFn, doc: "Bare invocation allowed. Subcommands: list, ls.", test_suffix: None },
        SubDef::Custom { name: "tasks", check: check_mise_tasks as CheckFn, doc: "Bare invocation allowed. Subcommands: deps, info, list, ls.", test_suffix: None },
        SubDef::Custom { name: "cache", check: check_mise_cache as CheckFn, doc: "Bare invocation allowed. Subcommands: path.", test_suffix: None },
        SubDef::Custom { name: "tool-alias", check: check_mise_tool_alias as CheckFn, doc: "Bare invocation allowed. Subcommands: get, list, ls.", test_suffix: None },
        SubDef::Custom { name: "shell-alias", check: check_mise_shell_alias as CheckFn, doc: "Bare invocation allowed. Subcommands: get, list, ls.", test_suffix: None },
        SubDef::Custom { name: "exec", check: check_mise_exec as CheckFn, doc: "exec delegates after --.", test_suffix: None },
        SubDef::Policy { name: "activate", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "deactivate", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "edit", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "en", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "generate", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "implode", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "install", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "link", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "lock", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "prepare", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "prune", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "run", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "self-update", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "shell", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "sync", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "uninstall", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "unset", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "unuse", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "upgrade", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "use", policy: &HELP_ONLY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "watch", policy: &HELP_ONLY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://mise.jdx.dev/cli/",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        mise_ls: "mise ls",
        mise_ls_current: "mise ls --current",
        mise_ls_json: "mise ls --json",
        mise_list: "mise list ruby",
        mise_current: "mise current ruby",
        mise_which: "mise which ruby",
        mise_where: "mise where ruby",
        mise_doctor: "mise doctor",
        mise_version: "mise --version",
        mise_settings_get: "mise settings get experimental",
        mise_settings_bare: "mise settings",
        mise_settings_ls: "mise settings ls",
        mise_settings_list: "mise settings list",
        mise_env: "mise env",
        mise_env_json: "mise env --json",
        mise_env_shell: "mise env --shell bash",
        mise_config_ls: "mise config ls",
        mise_config_list: "mise config list",
        mise_config_get: "mise config get experimental",
        mise_reshim: "mise reshim",
        mise_reshim_force: "mise reshim --force",
        mise_exec_git_status: "mise exec -- git status",
        mise_exec_bundle_rspec: "mise exec -- bundle exec rspec spec/foo_spec.rb --no-color",
        mise_outdated: "mise outdated",
        mise_outdated_json: "mise outdated --json",
        mise_search: "mise search node",
        mise_registry: "mise registry",
        mise_registry_backend: "mise registry --backend cargo",
        mise_latest: "mise latest ruby",
        mise_ls_remote: "mise ls-remote ruby",
        mise_ls_remote_all: "mise ls-remote ruby --all",
        mise_bin_paths: "mise bin-paths",
        mise_tool: "mise tool ruby",
        mise_completion: "mise completion bash",
        mise_trust_show: "mise trust --show",
        mise_fmt_check: "mise fmt --check",
        mise_set_bare: "mise set",
        mise_set_json: "mise set --json",
        mise_config_bare: "mise config",
        mise_plugins_bare: "mise plugins",
        mise_plugins_ls: "mise plugins ls",
        mise_plugins_list: "mise plugins list",
        mise_plugins_ls_remote: "mise plugins ls-remote",
        mise_backends_bare: "mise backends",
        mise_backends_ls: "mise backends ls",
        mise_backends_list: "mise backends list",
        mise_tasks_bare: "mise tasks",
        mise_tasks_ls: "mise tasks ls",
        mise_tasks_list: "mise tasks list",
        mise_tasks_info: "mise tasks info build",
        mise_tasks_deps: "mise tasks deps",
        mise_install_help: "mise install --help",
        mise_trust_help: "mise trust --help",
        mise_use_help: "mise use --help",
        mise_uninstall_help: "mise uninstall --help",
        mise_run_help: "mise run --help",
        mise_prune_help: "mise prune --help",
        mise_upgrade_help: "mise upgrade --help",
        mise_self_update_help: "mise self-update --help",
        mise_plugins_install_help: "mise plugins install --help",
        mise_tasks_run_help: "mise tasks run --help",
        mise_settings_help: "mise settings --help",
        mise_config_help: "mise config --help",
        mise_plugins_help: "mise plugins --help",
        mise_tasks_help: "mise tasks --help",
        mise_backends_help: "mise backends --help",
        mise_cache_bare: "mise cache",
        mise_cache_path: "mise cache path",
        mise_cache_help: "mise cache --help",
        mise_tool_alias_bare: "mise tool-alias",
        mise_tool_alias_ls: "mise tool-alias ls",
        mise_tool_alias_get: "mise tool-alias get node",
        mise_tool_alias_help: "mise tool-alias --help",
        mise_shell_alias_bare: "mise shell-alias",
        mise_shell_alias_ls: "mise shell-alias ls",
        mise_shell_alias_get: "mise shell-alias get ll",
        mise_shell_alias_help: "mise shell-alias --help",
    }

    denied! {
        mise_exec_node_version_denied: "mise exec node@20 -- node --version",
        mise_exec_rm_denied: "mise exec -- rm -rf /",
        mise_exec_no_inner_denied: "mise exec --",
        mise_exec_no_separator_denied: "mise exec ruby foo.rb",
        mise_exec_bare_denied: "mise exec",
        mise_exec_ruby_denied: "mise exec -- ruby foo.rb",
        mise_trust_bare_denied: "mise trust",
        mise_trust_untrust_denied: "mise trust --untrust",
        mise_trust_ignore_denied: "mise trust --ignore",
        mise_fmt_bare_denied: "mise fmt",
        mise_install_denied: "mise install ruby",
        mise_uninstall_denied: "mise uninstall ruby",
        mise_use_denied: "mise use ruby",
        mise_run_denied: "mise run build",
        mise_set_key_val_denied: "mise set FOO=bar",
        mise_settings_set_denied: "mise settings set experimental true",
        mise_plugins_install_denied: "mise plugins install ruby",
        mise_tasks_run_denied: "mise tasks run build",
        mise_cache_clear_denied: "mise cache clear",
        mise_cache_prune_denied: "mise cache prune",
        mise_tool_alias_set_denied: "mise tool-alias set node lts 20",
        mise_tool_alias_unset_denied: "mise tool-alias unset node lts",
        mise_shell_alias_set_denied: "mise shell-alias set ll ls -la",
        mise_shell_alias_unset_denied: "mise shell-alias unset ll",
    }
}
