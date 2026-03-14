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

static RAILS_ROUTES_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--expanded"]),
    valued: WordSet::flags(&[
        "--controller", "--grep", "-g",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static RAILS_TEST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--backtrace", "--color", "--defer-output", "--fail-fast",
        "--no-color", "--verbose",
        "-b", "-c", "-d", "-f", "-v",
    ]),
    valued: WordSet::flags(&[
        "--environment", "--name", "--seed",
        "-e", "-n", "-s",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static RAILS_NOTES_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[
        "--annotations", "-a",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static RAILS_BARE_SUBS: WordSet = WordSet::new(&[
    "about",
    "assets:reveal",
    "assets:reveal:full",
    "db:migrate:status",
    "db:version",
    "initializers",
    "middleware",
    "secret",
    "stats",
    "time:zones:all",
    "time:zones:local",
    "version",
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
    let Some(cmd) = tokens.get(1) else {
        return false;
    };
    if BUNDLE_EXEC_SAFE.contains(cmd) {
        return true;
    }
    if cmd == "rails" {
        return check_rails_sub(&tokens[1..]);
    }
    false
}

fn check_rails_sub(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let sub = &tokens[1];
    if tokens.len() == 2 && matches!(sub.as_str(), "--help" | "-h") {
        return true;
    }
    match sub.as_str() {
        "routes" => policy::check(&tokens[1..], &RAILS_ROUTES_POLICY),
        "test" | "test:system" => policy::check(&tokens[1..], &RAILS_TEST_POLICY),
        "notes" => policy::check(&tokens[1..], &RAILS_NOTES_POLICY),
        _ => RAILS_BARE_SUBS.contains(sub) && tokens.len() == 2,
    }
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
            doc: "exec allowed for: brakeman, cucumber, erb_lint, herb, rails (about, assets:reveal, assets:reveal:full, db:migrate:status, db:version, initializers, middleware, notes, routes, secret, stats, test, test:system, time:zones:all, time:zones:local, version), rspec, standardrb.",
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
    }

    denied! {
        bundle_exec_rails_console_denied: "bundle exec rails console",
        bundle_exec_rails_server_denied: "bundle exec rails server",
        bundle_exec_rails_generate_denied: "bundle exec rails generate model User",
        bundle_exec_rails_destroy_denied: "bundle exec rails destroy model User",
        bundle_exec_rails_db_migrate_denied: "bundle exec rails db:migrate",
        bundle_exec_rails_db_drop_denied: "bundle exec rails db:drop",
        bundle_exec_rails_db_seed_denied: "bundle exec rails db:seed",
        bundle_exec_rails_db_reset_denied: "bundle exec rails db:reset",
        bundle_exec_rails_db_setup_denied: "bundle exec rails db:setup",
        bundle_exec_rails_db_rollback_denied: "bundle exec rails db:rollback",
        bundle_exec_rails_db_create_denied: "bundle exec rails db:create",
        bundle_exec_rails_db_schema_load_denied: "bundle exec rails db:schema:load",
        bundle_exec_rails_runner_denied: "bundle exec rails runner script.rb",
        bundle_exec_rails_dbconsole_denied: "bundle exec rails dbconsole",
        bundle_exec_rails_credentials_edit_denied: "bundle exec rails credentials:edit",
        bundle_exec_rails_bare_denied: "bundle exec rails",
        bundle_exec_rake_denied: "bundle exec rake db:drop",
        bundle_exec_ruby_denied: "bundle exec ruby script.rb",
        bundle_config_set_denied: "bundle config set path vendor/bundle",
        bundle_config_unset_denied: "bundle config unset path",
        bundle_config_old_set_denied: "bundle config path vendor/bundle",
    }
}
