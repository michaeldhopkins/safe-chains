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
        "annotate"
        | "annotaterb"
        | "asciidoctor"
        | "brakeman"
        | "bridgetown"
        | "bundle-audit"
        | "bundler-audit"
        | "byebug"
        | "cucumber"
        | "danger"
        | "dawn"
        | "debride"
        | "erb_lint"
        | "erblint"
        | "erd"
        | "fasterer"
        | "flay"
        | "flog"
        | "foreman"
        | "fpm"
        | "gem"
        | "guard"
        | "haml"
        | "haml-lint"
        | "herb"
        | "i18n-tasks"
        | "jekyll"
        | "kamal"
        | "kramdown"
        | "license_finder"
        | "m"
        | "middleman"
        | "mutant"
        | "overcommit"
        | "packwerk"
        | "parallel_cucumber"
        | "parallel_rspec"
        | "parallel_spinach"
        | "parallel_test"
        | "pry"
        | "racc"
        | "railroady"
        | "rails"
        | "rake"
        | "rbs"
        | "rdoc"
        | "reek"
        | "rex"
        | "rspec"
        | "rubocop"
        | "rubycritic"
        | "ruby-audit"
        | "rufo"
        | "sdoc"
        | "slim-lint"
        | "slimrb"
        | "spring"
        | "srb"
        | "stackprof"
        | "standardrb"
        | "steep"
        | "stree"
        | "thor"
        | "typeprof"
        | "whenever"
        | "yard"
        | "yardoc"
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
        bundle_exec_reek: "bundle exec reek lib/",
        bundle_exec_flay: "bundle exec flay lib/",
        bundle_exec_flog_score: "bundle exec flog -s lib/",
        bundle_exec_fasterer: "bundle exec fasterer",
        bundle_exec_haml_lint: "bundle exec haml-lint app/views/",
        bundle_exec_haml_lint_autocorrect: "bundle exec haml-lint --auto-correct app/views/",
        bundle_exec_slim_lint: "bundle exec slim-lint app/views/",
        bundle_exec_bundler_audit_check: "bundle exec bundler-audit check",
        bundle_exec_bundle_audit_check: "bundle exec bundle-audit check",
        bundle_exec_ruby_audit_check: "bundle exec ruby-audit check",
        bundle_exec_yard_list: "bundle exec yard list",
        bundle_exec_yard_stats: "bundle exec yard stats",
        bundle_exec_yardoc: "bundle exec yardoc",
        bundle_exec_rdoc: "bundle exec rdoc lib",
        bundle_exec_rubycritic_no_browser: "bundle exec rubycritic --no-browser",
        bundle_exec_pry_version: "bundle exec pry --version",
        bundle_exec_byebug_version: "bundle exec byebug --version",
        bundle_exec_rake_help: "bundle exec rake --help",
        bundle_exec_steep_check: "bundle exec steep check",
        bundle_exec_steep_stats: "bundle exec steep stats",
        bundle_exec_srb_tc: "bundle exec srb tc",
        bundle_exec_srb_tc_autocorrect: "bundle exec srb tc -a",
        bundle_exec_rbs_validate: "bundle exec rbs validate",
        bundle_exec_rbs_list: "bundle exec rbs list --class",
        bundle_exec_rbs_collection_install: "bundle exec rbs collection install",
        bundle_exec_typeprof: "bundle exec typeprof app.rb",
        bundle_exec_typeprof_out: "bundle exec typeprof -o sig/app.rbs app.rb",
        bundle_exec_stree_check: "bundle exec stree check lib/",
        bundle_exec_stree_format: "bundle exec stree format lib/foo.rb",
        bundle_exec_stree_write: "bundle exec stree write lib/foo.rb",
        bundle_exec_rufo_check: "bundle exec rufo --check lib/",
        bundle_exec_rufo: "bundle exec rufo lib/",
        bundle_exec_packwerk_check: "bundle exec packwerk check",
        bundle_exec_packwerk_update_todo: "bundle exec packwerk update-todo",
        bundle_exec_debride: "bundle exec debride lib/",
        bundle_exec_i18n_tasks_missing: "bundle exec i18n-tasks missing",
        bundle_exec_i18n_tasks_health: "bundle exec i18n-tasks health",
        bundle_exec_i18n_tasks_normalize: "bundle exec i18n-tasks normalize",
        bundle_exec_i18n_tasks_remove_unused: "bundle exec i18n-tasks remove-unused",
        bundle_exec_asciidoctor: "bundle exec asciidoctor README.adoc",
        bundle_exec_kramdown: "bundle exec kramdown README.md",
        bundle_exec_dawn_scan: "bundle exec dawn scan app/",
        bundle_exec_stackprof: "bundle exec stackprof tmp/profile.dump",
        bundle_exec_fpm: "bundle exec fpm -s dir -t deb -n mypkg lib/",
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
        bundle_exec_pry_script_denied: "bundle exec pry script.rb",
        bundle_exec_byebug_script_denied: "bundle exec byebug script.rb",
        bundle_exec_rubycritic_no_no_browser_denied: "bundle exec rubycritic",
        bundle_exec_jekyll_build_denied: "bundle exec jekyll build",
        bundle_exec_jekyll_serve_denied: "bundle exec jekyll serve",
        bundle_exec_foreman_start_denied: "bundle exec foreman start",
        bundle_exec_guard_start_denied: "bundle exec guard start",
        bundle_exec_annotaterb_models_denied: "bundle exec annotaterb models",
        bundle_exec_thor_list_denied: "bundle exec thor list",
        bundle_exec_haml_lint_unknown_denied: "bundle exec haml-lint --xyzzy",
        bundle_exec_srb_init_denied: "bundle exec srb init",
        bundle_exec_srb_rbi_denied: "bundle exec srb rbi gems",
        bundle_exec_steep_watch_denied: "bundle exec steep watch",
        bundle_exec_steep_langserver_denied: "bundle exec steep langserver",
        bundle_exec_rbs_test_denied: "bundle exec rbs test",
        bundle_exec_stree_lsp_denied: "bundle exec stree lsp",
        bundle_exec_i18n_tasks_irb_denied: "bundle exec i18n-tasks irb",
        bundle_exec_kamal_deploy_denied: "bundle exec kamal deploy",
        bundle_exec_danger_pr_denied: "bundle exec danger pr",
        bundle_exec_mutant_run_denied: "bundle exec mutant run -- Foo",
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
