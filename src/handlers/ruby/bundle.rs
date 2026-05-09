use crate::verdict::{SafetyLevel, Verdict};
use crate::parse::{Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

static BUNDLE_CONFIG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(1),
    flag_style: FlagStyle::Strict,
    numeric_dash: false,
};

static BUNDLE_CONFIG_GET_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
    numeric_dash: false,
};

static HELP_ONLY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
    numeric_dash: false,
};

/// Legacy fast path for tools that don't yet have a top-level command
/// allowlist. New entries should land as a top-level TOML and be added
/// to the bundle-exec delegation match above instead — that path
/// recursively validates flags so write-mode flags promote the level
/// correctly. This list will shrink to empty as those land.
static BUNDLE_EXEC_SAFE: WordSet = WordSet::new(&[]);

pub fn check_bundle_config(tokens: &[Token]) -> Verdict {
    if tokens.len() == 1 {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let sub = tokens[1].as_str();
    if tokens.len() == 2 && (sub == "--help" || sub == "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    match sub {
        "get" | "list" if policy::check(&tokens[1..], &BUNDLE_CONFIG_GET_POLICY) => {
            return Verdict::Allowed(SafetyLevel::Inert);
        }
        "set" | "unset" => {
            if policy::check(&tokens[1..], &HELP_ONLY) {
                return Verdict::Allowed(SafetyLevel::Inert);
            }
            return Verdict::Denied;
        }
        _ => {}
    }
    if policy::check(tokens, &BUNDLE_CONFIG_POLICY) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }
}

pub fn check_bundle_exec(tokens: &[Token]) -> Verdict {
    let Some(cmd) = tokens.get(1) else {
        return Verdict::Denied;
    };
    if tokens.len() == 2 && (cmd == "--help" || cmd == "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    // Tools that are also covered as top-level commands: delegate
    // through command_verdict so flag-driven level promotion (e.g.
    // rubocop --autocorrect → SafeWrite) and full flag allowlists
    // apply, instead of the legacy hard-coded SafeRead shortcut.
    if matches!(
        cmd.as_str(),
        "brakeman"
        | "cucumber"
        | "erb_lint"
        | "erblint"
        | "gem"
        | "herb"
        | "rails"
        | "rspec"
        | "rubocop"
        | "standardrb"
    ) {
        let inner = shell_words::join(tokens[1..].iter().map(|t| t.as_str()));
        return crate::command_verdict(&inner);
    }
    if BUNDLE_EXEC_SAFE.contains(cmd) {
        return Verdict::Allowed(SafetyLevel::SafeRead);
    }
    if cmd == "appraisal" {
        return check_appraisal(&tokens[1..]);
    }
    Verdict::Denied
}

fn check_appraisal(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Denied;
    }
    let sub = &tokens[1];
    if tokens.len() == 2 && (sub == "--help" || sub == "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    if sub == "list" {
        return if tokens.len() == 2 { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }
    if tokens.len() < 3 {
        return Verdict::Denied;
    }
    check_bundle_exec(&tokens[1..])
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
        bundle_install: "bundle install",
        bundle_install_quiet: "bundle install --quiet",
        bundle_install_jobs: "bundle install --jobs 4",
        bundle_install_jobs_eq: "bundle install --jobs=4",
        bundle_install_jobs_short: "bundle install -j 4",
        bundle_install_gemfile: "bundle install --gemfile gemfiles/rails_7.gemfile",
        bundle_install_path: "bundle install --path vendor/bundle",
        bundle_install_local: "bundle install --local",
        bundle_install_deployment: "bundle install --deployment --without development test",
        bundle_install_retry: "bundle install --retry 3",
        bundle_install_clean: "bundle install --clean --quiet",
        bundle_install_frozen: "bundle install --frozen",
        bundle_install_with: "bundle install --with development",
        bundle_install_combined: "bundle install --quiet --jobs 4 --retry 3",
        bundle_exec_rspec: "bundle exec rspec spec/models/foo_spec.rb",
        bundle_exec_standardrb: "bundle exec standardrb app/models/foo.rb",
        bundle_exec_standardrb_fix: "bundle exec standardrb --fix app/models/foo.rb",
        bundle_exec_cucumber: "bundle exec cucumber",
        bundle_exec_brakeman: "bundle exec brakeman",
        bundle_exec_erb_lint: "bundle exec erb_lint app/views/foo.html.erb",
        bundle_exec_herb_lint: "bundle exec herb lint app/views/foo.html.erb",
        bundle_exec_herb_format_check: "bundle exec herb format --check app/views/foo.html.erb",
        bundle_exec_rails_routes: "bundle exec rails routes",
        bundle_exec_rails_routes_grep: "bundle exec rails routes --grep dun_and_bradstreet",
        bundle_exec_rails_routes_controller: "bundle exec rails routes --controller users",
        bundle_exec_rails_routes_expanded: "bundle exec rails routes --expanded",
        bundle_exec_rails_about: "bundle exec rails about",
        bundle_exec_rails_stats: "bundle exec rails stats",
        bundle_exec_rails_notes: "bundle exec rails notes",
        bundle_exec_rails_notes_annotations: "bundle exec rails notes --annotations FIXME RELEASE",
        bundle_exec_rails_version: "bundle exec rails version",
        bundle_exec_rails_help: "bundle exec rails --help",
        bundle_exec_rails_initializers: "bundle exec rails initializers",
        bundle_exec_rails_middleware: "bundle exec rails middleware",
        bundle_exec_rails_secret: "bundle exec rails secret",
        bundle_exec_rails_db_migrate_status: "bundle exec rails db:migrate:status",
        bundle_exec_rails_db_version: "bundle exec rails db:version",
        bundle_exec_rails_time_zones_all: "bundle exec rails time:zones:all",
        bundle_exec_rails_time_zones_local: "bundle exec rails time:zones:local",
        bundle_exec_rails_assets_reveal: "bundle exec rails assets:reveal",
        bundle_exec_rails_assets_reveal_full: "bundle exec rails assets:reveal:full",
        bundle_exec_rails_test: "bundle exec rails test",
        bundle_exec_rails_test_file: "bundle exec rails test test/models/user_test.rb",
        bundle_exec_rails_test_seed: "bundle exec rails test --seed 1234",
        bundle_exec_rails_test_verbose: "bundle exec rails test -v",
        bundle_exec_rails_test_fail_fast: "bundle exec rails test --fail-fast",
        bundle_exec_rails_test_name: "bundle exec rails test -n /user/",
        bundle_exec_rails_test_env: "bundle exec rails test -e test",
        bundle_exec_rails_test_system: "bundle exec rails test:system",
        bundle_config_bare: "bundle config",
        bundle_config_key: "bundle config path",
        bundle_config_list: "bundle config list",
        bundle_config_get: "bundle config get path",
        bundle_config_help: "bundle config --help",
        bundle_config_set_help: "bundle config set --help",
        bundle_version: "bundle --version",
        bundle_exec_gem_list: "bundle exec gem list",
        bundle_exec_gem_list_local: "bundle exec gem list --local",
        bundle_exec_gem_dependency: "bundle exec gem dependency rails",
        bundle_exec_gem_info: "bundle exec gem info rails",
        bundle_exec_gem_which: "bundle exec gem which bundler",
        bundle_exec_gem_environment: "bundle exec gem environment",
        bundle_exec_gem_contents: "bundle exec gem contents rails",
        bundle_exec_gem_search: "bundle exec gem search rails",
        bundle_exec_gem_outdated: "bundle exec gem outdated",
        bundle_exec_gem_help: "bundle exec gem --help",
        bundle_exec_gem_version: "bundle exec gem --version",
        bundle_exec_appraisal_list: "bundle exec appraisal list",
        bundle_exec_appraisal_help: "bundle exec appraisal --help",
        bundle_exec_appraisal_rspec: "bundle exec appraisal rails-7-1 rspec spec/models/foo_spec.rb",
        bundle_exec_appraisal_rspec_tag: "bundle exec appraisal rails-7-1 rspec --tag focus spec/",
        bundle_exec_appraisal_cucumber: "bundle exec appraisal rails-7-1 cucumber",
        bundle_exec_rails_db_create: "bundle exec rails db:create",
        bundle_exec_rails_db_migrate: "bundle exec rails db:migrate",
        bundle_exec_rails_db_seed: "bundle exec rails db:seed",
        bundle_exec_rails_db_setup: "bundle exec rails db:setup",
        bundle_exec_rails_db_schema_load: "bundle exec rails db:schema:load",
        bundle_exec_rails_generate: "bundle exec rails generate model User name:string",
    }

    denied! {
        bundle_exec_rails_console_denied: "bundle exec rails console",
        bundle_exec_rails_server_denied: "bundle exec rails server",
        bundle_exec_rails_destroy_denied: "bundle exec rails destroy model User",
        bundle_exec_rails_db_drop_denied: "bundle exec rails db:drop",
        bundle_exec_rails_db_reset_denied: "bundle exec rails db:reset",
        bundle_exec_rails_db_rollback_denied: "bundle exec rails db:rollback",
        bundle_exec_rails_runner_denied: "bundle exec rails runner script.rb",
        bundle_exec_rails_dbconsole_denied: "bundle exec rails dbconsole",
        bundle_exec_rails_credentials_edit_denied: "bundle exec rails credentials:edit",
        bundle_exec_rails_bare_denied: "bundle exec rails",
        bundle_exec_rake_denied: "bundle exec rake db:drop",
        bundle_exec_ruby_denied: "bundle exec ruby script.rb",
        bundle_config_set_denied: "bundle config set path vendor/bundle",
        bundle_config_unset_denied: "bundle config unset path",
        bundle_config_old_set_denied: "bundle config path vendor/bundle",
        bundle_exec_gem_install_denied: "bundle exec gem install rails",
        bundle_exec_gem_uninstall_denied: "bundle exec gem uninstall rails",
        bundle_exec_gem_update_denied: "bundle exec gem update",
        bundle_exec_gem_bare_denied: "bundle exec gem",
        bundle_exec_appraisal_bare_denied: "bundle exec appraisal",
        bundle_exec_appraisal_gemfile_only_denied: "bundle exec appraisal rails-7-1",
        bundle_exec_appraisal_rm_denied: "bundle exec appraisal rails-7-1 rm -rf /",
        bundle_exec_appraisal_list_extra_denied: "bundle exec appraisal list foo",
        bundle_exec_appraisal_list_flag_denied: "bundle exec appraisal list --unknown",
    }
}
