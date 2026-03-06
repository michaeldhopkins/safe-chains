use crate::command::{CommandDef, SubDef};
use crate::parse::{Segment, Token, WordSet};
use crate::policy::{FlagPolicy, FlagStyle};

static BUNDLE_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--name-only", "--paths"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BUNDLE_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--path"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BUNDLE_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--paths"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BUNDLE_CHECK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--dry-run"]),
    standalone_short: b"",
    valued: WordSet::new(&["--gemfile", "--path"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BUNDLE_EXEC_SAFE: WordSet = WordSet::new(&[
    "brakeman", "cucumber", "erb_lint", "herb", "rspec", "standardrb",
]);

fn check_bundle_exec(tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    tokens.get(1).is_some_and(|t| BUNDLE_EXEC_SAFE.contains(t))
}

pub(crate) static BUNDLE: CommandDef = CommandDef {
    name: "bundle",
    subs: &[
        SubDef::Policy { name: "check", policy: &BUNDLE_CHECK_POLICY },
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
};

static GEM_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--installed", "--local", "--no-details",
        "--no-versions", "--prerelease", "--remote", "--versions",
    ]),
    standalone_short: b"adilr",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GEM_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--installed", "--prerelease"]),
    standalone_short: b"i",
    valued: WordSet::new(&["--version"]),
    valued_short: b"v",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GEM_SEARCH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--details", "--exact", "--local",
        "--prerelease", "--remote", "--versions",
    ]),
    standalone_short: b"adeilr",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GEM_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--local", "--prerelease", "--remote", "--versions",
    ]),
    standalone_short: b"ailr",
    valued: WordSet::new(&["--version"]),
    valued_short: b"v",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static GEM: CommandDef = CommandDef {
    name: "gem",
    subs: &[
        SubDef::Policy { name: "contents", policy: &GEM_SIMPLE_POLICY },
        SubDef::Policy { name: "dependency", policy: &GEM_SIMPLE_POLICY },
        SubDef::Policy { name: "environment", policy: &GEM_SIMPLE_POLICY },
        SubDef::Policy { name: "help", policy: &GEM_SIMPLE_POLICY },
        SubDef::Policy { name: "info", policy: &GEM_INFO_POLICY },
        SubDef::Policy { name: "list", policy: &GEM_LIST_POLICY },
        SubDef::Policy { name: "outdated", policy: &GEM_SIMPLE_POLICY },
        SubDef::Policy { name: "pristine", policy: &GEM_SIMPLE_POLICY },
        SubDef::Policy { name: "search", policy: &GEM_SEARCH_POLICY },
        SubDef::Policy { name: "sources", policy: &GEM_SIMPLE_POLICY },
        SubDef::Policy { name: "specification", policy: &GEM_SIMPLE_POLICY },
        SubDef::Policy { name: "stale", policy: &GEM_SIMPLE_POLICY },
        SubDef::Policy { name: "which", policy: &GEM_SIMPLE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

static RBENV_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static RBENV: CommandDef = CommandDef {
    name: "rbenv",
    subs: &[
        SubDef::Policy { name: "help", policy: &RBENV_BARE_POLICY },
        SubDef::Policy { name: "root", policy: &RBENV_BARE_POLICY },
        SubDef::Policy { name: "shims", policy: &RBENV_BARE_POLICY },
        SubDef::Policy { name: "version", policy: &RBENV_BARE_POLICY },
        SubDef::Policy { name: "versions", policy: &RBENV_BARE_POLICY },
        SubDef::Policy { name: "which", policy: &RBENV_BARE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    BUNDLE.dispatch(cmd, tokens, is_safe)
        .or_else(|| GEM.dispatch(cmd, tokens, is_safe))
        .or_else(|| RBENV.dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![BUNDLE.to_doc(), GEM.to_doc(), RBENV.to_doc()]
}

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
        bundle_version: "bundle --version",
        gem_list: "gem list",
        gem_list_local: "gem list --local",
        gem_list_remote: "gem list --remote",
        gem_list_all: "gem list --all",
        gem_info: "gem info rails",
        gem_info_installed: "gem info rails --installed",
        gem_environment: "gem environment",
        gem_which: "gem which bundler",
        gem_pristine: "gem pristine --all",
        gem_search: "gem search rails",
        gem_search_remote: "gem search rails --remote",
        gem_search_exact: "gem search rails --exact",
        gem_specification: "gem specification rails",
        gem_dependency: "gem dependency rails",
        gem_contents: "gem contents rails",
        gem_sources: "gem sources",
        gem_stale: "gem stale",
        gem_outdated: "gem outdated",
        gem_help: "gem help",
        gem_version: "gem --version",
        rbenv_versions: "rbenv versions",
        rbenv_version: "rbenv version",
        rbenv_which: "rbenv which ruby",
        rbenv_root: "rbenv root",
        rbenv_shims: "rbenv shims",
        rbenv_version_flag: "rbenv --version",
        rbenv_help: "rbenv help",
    }

    denied! {
        bundle_exec_rails_console_denied: "bundle exec rails console",
        bundle_exec_rake_denied: "bundle exec rake db:drop",
        bundle_exec_ruby_denied: "bundle exec ruby script.rb",
    }
}
