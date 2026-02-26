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
    use crate::docs::{CommandDoc, describe_wordset};
    vec![
        CommandDoc::handler("bundle", format!(
            "{} Guarded: exec ({} only).",
            describe_wordset(&BUNDLE_READ_ONLY),
            BUNDLE_EXEC_SAFE.iter().collect::<Vec<_>>().join(", "),
        )),
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

    #[test]
    fn bundle_list() {
        assert!(check("bundle list"));
    }

    #[test]
    fn bundle_info() {
        assert!(check("bundle info rails"));
    }

    #[test]
    fn bundle_show() {
        assert!(check("bundle show actionpack"));
    }

    #[test]
    fn bundle_check() {
        assert!(check("bundle check"));
    }

    #[test]
    fn bundle_exec_rspec() {
        assert!(check("bundle exec rspec spec/models/foo_spec.rb"));
    }

    #[test]
    fn bundle_exec_standardrb() {
        assert!(check("bundle exec standardrb app/models/foo.rb"));
    }

    #[test]
    fn bundle_exec_standardrb_fix() {
        assert!(check("bundle exec standardrb --fix app/models/foo.rb"));
    }

    #[test]
    fn bundle_exec_cucumber() {
        assert!(check("bundle exec cucumber"));
    }

    #[test]
    fn bundle_exec_brakeman() {
        assert!(check("bundle exec brakeman"));
    }

    #[test]
    fn bundle_exec_erb_lint() {
        assert!(check("bundle exec erb_lint app/views/foo.html.erb"));
    }

    #[test]
    fn bundle_exec_herb() {
        assert!(check("bundle exec herb app/views/foo.html.erb"));
    }

    #[test]
    fn bundle_version() {
        assert!(check("bundle --version"));
    }

    #[test]
    fn bundle_install_denied() {
        assert!(!check("bundle install"));
    }

    #[test]
    fn bundle_update_denied() {
        assert!(!check("bundle update"));
    }

    #[test]
    fn bundle_exec_rails_console_denied() {
        assert!(!check("bundle exec rails console"));
    }

    #[test]
    fn bundle_exec_rake_denied() {
        assert!(!check("bundle exec rake db:drop"));
    }

    #[test]
    fn bundle_exec_ruby_denied() {
        assert!(!check("bundle exec ruby script.rb"));
    }

    #[test]
    fn gem_list() {
        assert!(check("gem list"));
    }

    #[test]
    fn gem_info() {
        assert!(check("gem info rails"));
    }

    #[test]
    fn gem_environment() {
        assert!(check("gem environment"));
    }

    #[test]
    fn gem_which() {
        assert!(check("gem which bundler"));
    }

    #[test]
    fn gem_pristine() {
        assert!(check("gem pristine --all"));
    }

    #[test]
    fn gem_search() {
        assert!(check("gem search rails"));
    }

    #[test]
    fn gem_specification() {
        assert!(check("gem specification rails"));
    }

    #[test]
    fn gem_dependency() {
        assert!(check("gem dependency rails"));
    }

    #[test]
    fn gem_contents() {
        assert!(check("gem contents rails"));
    }

    #[test]
    fn gem_sources() {
        assert!(check("gem sources"));
    }

    #[test]
    fn gem_stale() {
        assert!(check("gem stale"));
    }

    #[test]
    fn gem_outdated() {
        assert!(check("gem outdated"));
    }

    #[test]
    fn gem_help() {
        assert!(check("gem help"));
    }

    #[test]
    fn gem_version() {
        assert!(check("gem --version"));
    }

    #[test]
    fn gem_install_denied() {
        assert!(!check("gem install rails"));
    }

    #[test]
    fn gem_uninstall_denied() {
        assert!(!check("gem uninstall rails"));
    }

    #[test]
    fn rbenv_versions() {
        assert!(check("rbenv versions"));
    }

    #[test]
    fn rbenv_version() {
        assert!(check("rbenv version"));
    }

    #[test]
    fn rbenv_which() {
        assert!(check("rbenv which ruby"));
    }

    #[test]
    fn rbenv_root() {
        assert!(check("rbenv root"));
    }

    #[test]
    fn rbenv_shims() {
        assert!(check("rbenv shims"));
    }

    #[test]
    fn rbenv_version_flag() {
        assert!(check("rbenv --version"));
    }

    #[test]
    fn rbenv_help() {
        assert!(check("rbenv help"));
    }

    #[test]
    fn rbenv_install_denied() {
        assert!(!check("rbenv install 3.3.0"));
    }

    #[test]
    fn rbenv_global_denied() {
        assert!(!check("rbenv global 3.3.0"));
    }

    #[test]
    fn rbenv_local_denied() {
        assert!(!check("rbenv local 3.3.0"));
    }
}
