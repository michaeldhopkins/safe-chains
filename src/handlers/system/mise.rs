use crate::command::{CheckFn, CommandDef, SubDef};
use crate::parse::{Segment, Token, WordSet};
use crate::policy::{FlagPolicy, FlagStyle};

static MISE_RESHIM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--force"]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static MISE_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"qv",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static MISE_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--current", "--installed", "--json", "--missing",
        "--no-header", "--prefix",
    ]),
    standalone_short: b"ciJm",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static MISE_ENV_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json"]),
    standalone_short: b"J",
    valued: WordSet::flags(&["--shell"]),
    valued_short: b"s",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn check_mise_exec(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    let sep = tokens[1..].iter().position(|t| *t == "--");
    if let Some(pos) = sep {
        let inner_start = 1 + pos + 1;
        if inner_start >= tokens.len() {
            return false;
        }
        let inner = Token::join(&tokens[inner_start..]);
        return is_safe(&inner);
    }
    false
}

pub(crate) static MISE: CommandDef = CommandDef {
    name: "mise",
    subs: &[
        SubDef::Policy { name: "list", policy: &MISE_LIST_POLICY },
        SubDef::Policy { name: "ls", policy: &MISE_LIST_POLICY },
        SubDef::Policy { name: "current", policy: &MISE_SIMPLE_POLICY },
        SubDef::Policy { name: "which", policy: &MISE_SIMPLE_POLICY },
        SubDef::Policy { name: "doctor", policy: &MISE_SIMPLE_POLICY },
        SubDef::Policy { name: "reshim", policy: &MISE_RESHIM_POLICY },
        SubDef::Policy { name: "env", policy: &MISE_ENV_POLICY },
        SubDef::Nested { name: "config", subs: &[
            SubDef::Policy { name: "list", policy: &MISE_SIMPLE_POLICY },
            SubDef::Policy { name: "ls", policy: &MISE_SIMPLE_POLICY },
        ]},
        SubDef::Nested { name: "settings", subs: &[
            SubDef::Policy { name: "get", policy: &MISE_SIMPLE_POLICY },
        ]},
        SubDef::Custom { name: "exec", check: check_mise_exec as CheckFn, doc: "exec delegates after --.", test_suffix: None },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://mise.jdx.dev/cli/",
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
        mise_doctor: "mise doctor",
        mise_version: "mise --version",
        mise_settings_get: "mise settings get experimental",
        mise_env: "mise env",
        mise_env_json: "mise env --json",
        mise_env_shell: "mise env --shell bash",
        mise_config_ls: "mise config ls",
        mise_config_list: "mise config list",
        mise_reshim: "mise reshim",
        mise_reshim_force: "mise reshim --force",
        mise_exec_git_status: "mise exec -- git status",
        mise_exec_bundle_rspec: "mise exec -- bundle exec rspec spec/foo_spec.rb --no-color",
    }

    denied! {
        mise_exec_node_version_denied: "mise exec node@20 -- node --version",
        mise_exec_rm_denied: "mise exec -- rm -rf /",
        mise_exec_no_inner_denied: "mise exec --",
        mise_exec_no_separator_denied: "mise exec ruby foo.rb",
        mise_exec_bare_denied: "mise exec",
        mise_exec_ruby_denied: "mise exec -- ruby foo.rb",
        mise_config_bare_denied: "mise config",
    }
}
