use crate::command::{CommandDef, SubDef};
use crate::parse::{Segment, Token, WordSet};
use crate::policy::{FlagPolicy, FlagStyle};

static BUNDLE_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--name-only", "--paths"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BUNDLE_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--path"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BUNDLE_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--paths"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BUNDLE_CHECK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--dry-run"]),
    valued: WordSet::flags(&["--gemfile", "--path"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BUNDLE_CONFIG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(1),
    flag_style: FlagStyle::Strict,
};

static BUNDLE_CONFIG_GET_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BUNDLE_EXEC_SAFE: WordSet = WordSet::new(&[
    "brakeman", "cucumber", "erb_lint", "herb", "rspec", "standardrb",
]);

use crate::command::CheckFn;
use crate::policy;

static CONFIG_SUBS: &[SubDef] = &[
    SubDef::Policy { name: "get", policy: &BUNDLE_CONFIG_GET_POLICY },
    SubDef::Policy { name: "list", policy: &BUNDLE_CONFIG_GET_POLICY },
    SubDef::Policy { name: "set", policy: &HELP_ONLY },
    SubDef::Policy { name: "unset", policy: &HELP_ONLY },
];

static HELP_ONLY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

fn check_bundle_config(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    if tokens.len() == 1 {
        return true;
    }
    let sub = tokens[1].as_str();
    if tokens.len() == 2 && (sub == "--help" || sub == "-h") {
        return true;
    }
    if CONFIG_SUBS.iter().any(|s| s.name() == sub && s.check(&tokens[1..], is_safe)) {
        return true;
    }
    policy::check(tokens, &BUNDLE_CONFIG_POLICY)
}

fn check_bundle_exec(tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    tokens.get(1).is_some_and(|t| BUNDLE_EXEC_SAFE.contains(t))
}

pub(crate) static BUNDLE: CommandDef = CommandDef {
    name: "bundle",
    subs: &[
        SubDef::Policy { name: "check", policy: &BUNDLE_CHECK_POLICY },
        SubDef::Custom {
            name: "config",
            check: check_bundle_config as CheckFn,
            doc: "Bare and single-key lookup allowed. Subcommands: get, list.",
            test_suffix: None,
        },
        SubDef::Custom {
            name: "exec",
            check: check_bundle_exec,
            doc: "exec allowed for: brakeman, cucumber, erb_lint, herb, rspec, standardrb.",
            test_suffix: None,
        },
        SubDef::Policy { name: "info", policy: &BUNDLE_INFO_POLICY },
        SubDef::Policy { name: "list", policy: &BUNDLE_LIST_POLICY },
        SubDef::Policy { name: "show", policy: &BUNDLE_SHOW_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://bundler.io/man/bundle.1.html",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        bundle_list: "bundle list",
        bundle_list_name_only: "bundle list --name-only",
        bundle_list_paths: "bundle list --paths",
        bundle_info: "bundle info rails",
        bundle_info_path: "bundle info rails --path",
        bundle_show: "bundle show actionpack",
        bundle_show_paths: "bundle show --paths",
        bundle_check: "bundle check",
        bundle_check_dry_run: "bundle check --dry-run",
        bundle_exec_rspec: "bundle exec rspec spec/models/foo_spec.rb",
        bundle_exec_standardrb: "bundle exec standardrb app/models/foo.rb",
        bundle_exec_standardrb_fix: "bundle exec standardrb --fix app/models/foo.rb",
        bundle_exec_cucumber: "bundle exec cucumber",
        bundle_exec_brakeman: "bundle exec brakeman",
        bundle_exec_erb_lint: "bundle exec erb_lint app/views/foo.html.erb",
        bundle_exec_herb: "bundle exec herb app/views/foo.html.erb",
        bundle_config_bare: "bundle config",
        bundle_config_key: "bundle config path",
        bundle_config_list: "bundle config list",
        bundle_config_get: "bundle config get path",
        bundle_config_help: "bundle config --help",
        bundle_config_set_help: "bundle config set --help",
        bundle_version: "bundle --version",
    }

    denied! {
        bundle_exec_rails_console_denied: "bundle exec rails console",
        bundle_exec_rake_denied: "bundle exec rake db:drop",
        bundle_exec_ruby_denied: "bundle exec ruby script.rb",
        bundle_config_set_denied: "bundle config set path vendor/bundle",
        bundle_config_unset_denied: "bundle config unset path",
        bundle_config_old_set_denied: "bundle config path vendor/bundle",
    }
}
