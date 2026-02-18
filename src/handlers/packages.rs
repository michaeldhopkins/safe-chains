use std::collections::HashSet;
use std::sync::LazyLock;

static YARN_READ_ONLY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["list", "info", "why", "--version"]));

static NPM_READ_ONLY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["view", "info", "list", "ls"]));

static PIP_READ_ONLY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["list", "show", "freeze", "check"]));

static BUNDLE_READ_ONLY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["list", "info", "show", "check"]));

static BUNDLE_EXEC_SAFE: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "rspec",
        "standardrb",
        "cucumber",
        "brakeman",
        "erb_lint",
        "herb",
    ])
});

static GEM_READ_ONLY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["list", "info", "environment", "which", "pristine"]));

static BREW_READ_ONLY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["list", "info", "--version"]));

static CARGO_SAFE: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "clippy",
        "test",
        "build",
        "check",
        "doc",
        "search",
        "--version",
        "bench",
    ])
});

static NPX_SAFE: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["eslint", "@herb-tools/linter", "karma"]));

static NPX_FLAGS_NO_ARG: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["--yes", "-y", "--no", "--ignore-existing", "-q", "--quiet"]));

static MISE_READ_ONLY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["ls", "list", "current", "which", "doctor", "--version"]));

static MISE_MULTI: LazyLock<Vec<(&'static str, HashSet<&'static str>)>> =
    LazyLock::new(|| vec![("settings", HashSet::from(["get"]))]);

static ASDF_READ_ONLY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["current", "which", "help", "list", "--version"]));

pub fn is_safe_yarn(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if YARN_READ_ONLY.contains(tokens[1].as_str()) {
        return true;
    }
    tokens[1] == "test" || tokens[1].starts_with("test:")
}

pub fn is_safe_npm(tokens: &[String]) -> bool {
    tokens.len() >= 2 && NPM_READ_ONLY.contains(tokens[1].as_str())
}

pub fn is_safe_pip(tokens: &[String]) -> bool {
    tokens.len() >= 2 && PIP_READ_ONLY.contains(tokens[1].as_str())
}

pub fn is_safe_bundle(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if BUNDLE_READ_ONLY.contains(tokens[1].as_str()) {
        return true;
    }
    tokens[1] == "exec"
        && tokens
            .get(2)
            .is_some_and(|t| BUNDLE_EXEC_SAFE.contains(t.as_str()))
}

pub fn is_safe_gem(tokens: &[String]) -> bool {
    tokens.len() >= 2 && GEM_READ_ONLY.contains(tokens[1].as_str())
}

pub fn is_safe_brew(tokens: &[String]) -> bool {
    tokens.len() >= 2 && BREW_READ_ONLY.contains(tokens[1].as_str())
}

pub fn is_safe_cargo(tokens: &[String]) -> bool {
    tokens.len() >= 2 && CARGO_SAFE.contains(tokens[1].as_str())
}

pub fn is_safe_npx(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let mut i = 1;
    while i < tokens.len() {
        if tokens[i] == "--package" || tokens[i] == "-p" {
            i += 2;
            continue;
        }
        if NPX_FLAGS_NO_ARG.contains(tokens[i].as_str()) {
            i += 1;
            continue;
        }
        if tokens[i] == "--" {
            i += 1;
            break;
        }
        if tokens[i].starts_with('-') {
            return false;
        }
        break;
    }
    i < tokens.len() && NPX_SAFE.contains(tokens[i].as_str())
}

pub fn is_safe_mise(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if MISE_READ_ONLY.contains(tokens[1].as_str()) {
        return true;
    }
    for (prefix, actions) in MISE_MULTI.iter() {
        if tokens[1] == *prefix {
            return tokens.get(2).is_some_and(|a| actions.contains(a.as_str()));
        }
    }
    false
}

pub fn is_safe_asdf(tokens: &[String]) -> bool {
    tokens.len() >= 2 && ASDF_READ_ONLY.contains(tokens[1].as_str())
}

#[cfg(test)]
mod tests {
    use crate::is_safe;

    fn check(cmd: &str) -> bool {
        is_safe(cmd)
    }

    #[test]
    fn yarn_list() {
        assert!(check("yarn list --depth=0"));
    }

    #[test]
    fn yarn_info() {
        assert!(check("yarn info react"));
    }

    #[test]
    fn yarn_why() {
        assert!(check("yarn why lodash"));
    }

    #[test]
    fn yarn_version() {
        assert!(check("yarn --version"));
    }

    #[test]
    fn yarn_test() {
        assert!(check("yarn test"));
    }

    #[test]
    fn yarn_test_watch() {
        assert!(check("yarn test:watch"));
    }

    #[test]
    fn yarn_test_with_args() {
        assert!(check("yarn test --testPathPattern=Foo"));
    }

    #[test]
    fn yarn_install_denied() {
        assert!(!check("yarn install"));
    }

    #[test]
    fn yarn_add_denied() {
        assert!(!check("yarn add react"));
    }

    #[test]
    fn yarn_remove_denied() {
        assert!(!check("yarn remove lodash"));
    }

    #[test]
    fn yarn_upgrade_denied() {
        assert!(!check("yarn upgrade"));
    }

    #[test]
    fn npm_view() {
        assert!(check("npm view react version"));
    }

    #[test]
    fn npm_info() {
        assert!(check("npm info lodash"));
    }

    #[test]
    fn npm_list() {
        assert!(check("npm list --depth=0"));
    }

    #[test]
    fn npm_ls() {
        assert!(check("npm ls"));
    }

    #[test]
    fn npm_install_denied() {
        assert!(!check("npm install react"));
    }

    #[test]
    fn npm_uninstall_denied() {
        assert!(!check("npm uninstall lodash"));
    }

    #[test]
    fn npm_run_denied() {
        assert!(!check("npm run build"));
    }

    #[test]
    fn npm_test_denied() {
        assert!(!check("npm test"));
    }

    #[test]
    fn pip_list() {
        assert!(check("pip list"));
    }

    #[test]
    fn pip_show() {
        assert!(check("pip show requests"));
    }

    #[test]
    fn pip_freeze() {
        assert!(check("pip freeze"));
    }

    #[test]
    fn pip_check() {
        assert!(check("pip check"));
    }

    #[test]
    fn pip3_list() {
        assert!(check("pip3 list"));
    }

    #[test]
    fn pip3_show() {
        assert!(check("pip3 show flask"));
    }

    #[test]
    fn pip3_freeze() {
        assert!(check("pip3 freeze"));
    }

    #[test]
    fn pip_install_denied() {
        assert!(!check("pip install requests"));
    }

    #[test]
    fn pip_uninstall_denied() {
        assert!(!check("pip uninstall flask"));
    }

    #[test]
    fn pip3_install_denied() {
        assert!(!check("pip3 install django"));
    }

    #[test]
    fn bare_pip_denied() {
        assert!(!check("pip"));
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
    fn gem_install_denied() {
        assert!(!check("gem install rails"));
    }

    #[test]
    fn gem_uninstall_denied() {
        assert!(!check("gem uninstall rails"));
    }

    #[test]
    fn brew_list() {
        assert!(check("brew list"));
    }

    #[test]
    fn brew_info() {
        assert!(check("brew info node"));
    }

    #[test]
    fn brew_version() {
        assert!(check("brew --version"));
    }

    #[test]
    fn brew_install_denied() {
        assert!(!check("brew install node"));
    }

    #[test]
    fn brew_uninstall_denied() {
        assert!(!check("brew uninstall node"));
    }

    #[test]
    fn brew_services_denied() {
        assert!(!check("brew services list"));
    }

    #[test]
    fn cargo_clippy() {
        assert!(check("cargo clippy -- -D warnings"));
    }

    #[test]
    fn cargo_test() {
        assert!(check("cargo test"));
    }

    #[test]
    fn cargo_build() {
        assert!(check("cargo build --release"));
    }

    #[test]
    fn cargo_check() {
        assert!(check("cargo check"));
    }

    #[test]
    fn cargo_doc() {
        assert!(check("cargo doc"));
    }

    #[test]
    fn cargo_search() {
        assert!(check("cargo search serde"));
    }

    #[test]
    fn cargo_version() {
        assert!(check("cargo --version"));
    }

    #[test]
    fn cargo_bench() {
        assert!(check("cargo bench"));
    }

    #[test]
    fn cargo_install_denied() {
        assert!(!check("cargo install --path ."));
    }

    #[test]
    fn cargo_run_denied() {
        assert!(!check("cargo run"));
    }

    #[test]
    fn cargo_clean_denied() {
        assert!(!check("cargo clean"));
    }

    #[test]
    fn npx_herb_linter() {
        assert!(check("npx @herb-tools/linter app/views/foo.html.erb"));
    }

    #[test]
    fn npx_eslint() {
        assert!(check("npx eslint src/"));
    }

    #[test]
    fn npx_karma() {
        assert!(check("npx karma start"));
    }

    #[test]
    fn npx_yes_flag() {
        assert!(check("npx --yes eslint src/"));
    }

    #[test]
    fn npx_y_flag() {
        assert!(check("npx -y @herb-tools/linter ."));
    }

    #[test]
    fn npx_package_flag() {
        assert!(check(
            "npx --package @herb-tools/linter @herb-tools/linter ."
        ));
    }

    #[test]
    fn npx_double_dash() {
        assert!(check("npx -- eslint src/"));
    }

    #[test]
    fn npx_react_scripts_denied() {
        assert!(!check("npx react-scripts start"));
    }

    #[test]
    fn npx_cowsay_denied() {
        assert!(!check("npx cowsay hello"));
    }

    #[test]
    fn bare_npx_denied() {
        assert!(!check("npx"));
    }

    #[test]
    fn npx_only_flags_denied() {
        assert!(!check("npx --yes"));
    }

    #[test]
    fn mise_ls() {
        assert!(check("mise ls"));
    }

    #[test]
    fn mise_list() {
        assert!(check("mise list ruby"));
    }

    #[test]
    fn mise_current() {
        assert!(check("mise current ruby"));
    }

    #[test]
    fn mise_which() {
        assert!(check("mise which ruby"));
    }

    #[test]
    fn mise_doctor() {
        assert!(check("mise doctor"));
    }

    #[test]
    fn mise_version() {
        assert!(check("mise --version"));
    }

    #[test]
    fn mise_settings_get() {
        assert!(check("mise settings get experimental"));
    }

    #[test]
    fn mise_install_denied() {
        assert!(!check("mise install ruby@3.4"));
    }

    #[test]
    fn mise_exec_denied() {
        assert!(!check("mise exec -- ruby foo.rb"));
    }

    #[test]
    fn mise_use_denied() {
        assert!(!check("mise use ruby@3.4"));
    }

    #[test]
    fn asdf_current() {
        assert!(check("asdf current ruby"));
    }

    #[test]
    fn asdf_which() {
        assert!(check("asdf which ruby"));
    }

    #[test]
    fn asdf_help() {
        assert!(check("asdf help"));
    }

    #[test]
    fn asdf_list() {
        assert!(check("asdf list ruby"));
    }

    #[test]
    fn asdf_version() {
        assert!(check("asdf --version"));
    }

    #[test]
    fn asdf_install_denied() {
        assert!(!check("asdf install ruby 3.4"));
    }
}
