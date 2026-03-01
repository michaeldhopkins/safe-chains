use crate::parse::{Token, WordSet};

static BUNDLE_READ_ONLY: WordSet =
    WordSet::new(&["--version", "check", "info", "list", "show"]);

static BUNDLE_EXEC_SAFE: WordSet = WordSet::new(&[
    "brakeman", "cucumber", "erb_lint", "herb", "rspec", "standardrb",
]);

static GEM_READ_ONLY: WordSet = WordSet::new(&[
    "--version", "contents", "dependency", "environment", "help", "info",
    "list", "outdated", "pristine", "search", "sources", "specification",
    "stale", "which",
]);

static RBENV_SAFE: WordSet = WordSet::new(&[
    "--version", "help", "root", "shims", "version", "versions", "which",
]);

pub fn is_safe_bundle(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if BUNDLE_READ_ONLY.contains(&tokens[1]) {
        return true;
    }
    tokens[1] == "exec"
        && tokens
            .get(2)
            .is_some_and(|t| BUNDLE_EXEC_SAFE.contains(t))
}

pub fn is_safe_gem(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && GEM_READ_ONLY.contains(&tokens[1])
}

pub fn is_safe_rbenv(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && RBENV_SAFE.contains(&tokens[1])
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, doc};
    vec![
        CommandDoc::handler("bundle",
            doc(&BUNDLE_READ_ONLY)
                .section(format!("Guarded: exec ({} only).",
                    BUNDLE_EXEC_SAFE.iter().collect::<Vec<_>>().join(", ")))
                .build()),
        CommandDoc::wordset("gem", &GEM_READ_ONLY),
        CommandDoc::wordset("rbenv", &RBENV_SAFE),
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        bundle_list: "bundle list",
        bundle_info: "bundle info rails",
        bundle_show: "bundle show actionpack",
        bundle_check: "bundle check",
        bundle_exec_rspec: "bundle exec rspec spec/models/foo_spec.rb",
        bundle_exec_standardrb: "bundle exec standardrb app/models/foo.rb",
        bundle_exec_standardrb_fix: "bundle exec standardrb --fix app/models/foo.rb",
        bundle_exec_cucumber: "bundle exec cucumber",
        bundle_exec_brakeman: "bundle exec brakeman",
        bundle_exec_erb_lint: "bundle exec erb_lint app/views/foo.html.erb",
        bundle_exec_herb: "bundle exec herb app/views/foo.html.erb",
        bundle_version: "bundle --version",
        gem_list: "gem list",
        gem_info: "gem info rails",
        gem_environment: "gem environment",
        gem_which: "gem which bundler",
        gem_pristine: "gem pristine --all",
        gem_search: "gem search rails",
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
