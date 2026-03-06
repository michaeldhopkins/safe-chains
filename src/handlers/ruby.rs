use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

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

pub fn is_safe_bundle(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    match tokens[1].as_str() {
        "list" => policy::check(&tokens[1..], &BUNDLE_LIST_POLICY),
        "info" => policy::check(&tokens[1..], &BUNDLE_INFO_POLICY),
        "show" => policy::check(&tokens[1..], &BUNDLE_SHOW_POLICY),
        "check" => policy::check(&tokens[1..], &BUNDLE_CHECK_POLICY),
        "exec" => tokens
            .get(2)
            .is_some_and(|t| BUNDLE_EXEC_SAFE.contains(t)),
        _ => false,
    }
}

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

pub fn is_safe_gem(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let policy = match tokens[1].as_str() {
        "list" => &GEM_LIST_POLICY,
        "info" => &GEM_INFO_POLICY,
        "search" => &GEM_SEARCH_POLICY,
        "contents" | "dependency" | "environment" | "help" | "outdated"
        | "pristine" | "sources" | "specification" | "stale" | "which" => &GEM_SIMPLE_POLICY,
        _ => return false,
    };
    policy::check(&tokens[1..], policy)
}

static RBENV_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_rbenv(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    static RBENV_SAFE: WordSet = WordSet::new(&[
        "help", "root", "shims", "version", "versions", "which",
    ]);
    if !RBENV_SAFE.contains(&tokens[1]) {
        return false;
    }
    policy::check(&tokens[1..], &RBENV_BARE_POLICY)
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "bundle" => Some(is_safe_bundle(tokens)),
        "gem" => Some(is_safe_gem(tokens)),
        "rbenv" => Some(is_safe_rbenv(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocBuilder, wordset_items};
    vec![
        CommandDoc::handler("bundle",
            DocBuilder::new()
                .section("Subcommands: check, info, list, show.")
                .section(format!("exec allowed for: {}.",
                    wordset_items(&BUNDLE_EXEC_SAFE)))
                .build()),
        CommandDoc::handler("gem",
            "Subcommands: contents, dependency, environment, help, info, list, outdated, \
             pristine, search, sources, specification, stale, which. \
            "),
        CommandDoc::handler("rbenv",
            "Subcommands: help, root, shims, version, versions, which."),
    ]
}

#[cfg(test)]
pub(super) const REGISTRY: &[super::CommandEntry] = &[
    super::CommandEntry::Subcommand { cmd: "bundle", subs: &[
        super::SubEntry::Policy { name: "list" },
        super::SubEntry::Policy { name: "info" },
        super::SubEntry::Policy { name: "show" },
        super::SubEntry::Policy { name: "check" },
        super::SubEntry::Delegation { name: "exec" },
    ]},
    super::CommandEntry::Subcommand { cmd: "gem", subs: &[
        super::SubEntry::Policy { name: "list" },
        super::SubEntry::Policy { name: "info" },
        super::SubEntry::Policy { name: "search" },
        super::SubEntry::Policy { name: "contents" },
        super::SubEntry::Policy { name: "dependency" },
        super::SubEntry::Policy { name: "environment" },
        super::SubEntry::Policy { name: "help" },
        super::SubEntry::Policy { name: "outdated" },
        super::SubEntry::Policy { name: "pristine" },
        super::SubEntry::Policy { name: "sources" },
        super::SubEntry::Policy { name: "specification" },
        super::SubEntry::Policy { name: "stale" },
        super::SubEntry::Policy { name: "which" },
    ]},
    super::CommandEntry::Subcommand { cmd: "rbenv", subs: &[
        super::SubEntry::Policy { name: "help" },
        super::SubEntry::Policy { name: "root" },
        super::SubEntry::Policy { name: "shims" },
        super::SubEntry::Policy { name: "version" },
        super::SubEntry::Policy { name: "versions" },
        super::SubEntry::Policy { name: "which" },
    ]},
];

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
        bundle_install_denied: "bundle install",
        bundle_update_denied: "bundle update",
        bundle_exec_rails_console_denied: "bundle exec rails console",
        bundle_exec_rake_denied: "bundle exec rake db:drop",
        bundle_exec_ruby_denied: "bundle exec ruby script.rb",
        gem_install_denied: "gem install rails",
        gem_uninstall_denied: "gem uninstall rails",
        rbenv_install_denied: "rbenv install 3.3.0",
        rbenv_global_denied: "rbenv global 3.3.0",
        rbenv_local_denied: "rbenv local 3.3.0",
    }
}
