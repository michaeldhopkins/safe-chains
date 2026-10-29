//! Bundler evaluates the Gemfile as Ruby for every subcommand that loads the bundle, and finds it by
//! walking up from the folder it runs in. A project with no Gemfile of its own loads whatever
//! Gemfile a folder above it holds, and `bundle exec` loads it before the command it runs.

use safe_chains::pathctx::{PathCtx, enter};

fn approved_in(root: &std::path::Path, command: &str) -> bool {
    let root = root.to_string_lossy().into_owned();
    let _ctx = enter(PathCtx { cwd: Some(root.clone()), root: Some(root), session_id: None });
    safe_chains::is_safe_command(command)
}

struct Layout {
    _dir: tempfile::TempDir,
    outer: std::path::PathBuf,
}

fn layout() -> Layout {
    let dir = tempfile::tempdir().expect("tempdir");
    let base = std::fs::canonicalize(dir.path()).expect("canonical tempdir");
    let outer = base.join("outer");
    std::fs::create_dir_all(outer.join("bare/src")).expect("bare project");
    std::fs::create_dir_all(outer.join("ruby/src")).expect("ruby project");
    std::fs::write(outer.join("Gemfile"), "system('id')\n").expect("outer Gemfile");
    std::fs::write(outer.join("ruby/Gemfile"), "source 'https://rubygems.org'\n").expect("project Gemfile");
    Layout { _dir: dir, outer }
}

const LOADS_THE_BUNDLE: [&str; 8] = [
    "bundle list", "bundle check", "bundle show", "bundle info rake", "bundle install", "bundle exec gem --version",
    "bundle exec rake --version", "bundle exec rspec --version",
];

#[test]
fn a_gemfile_found_above_the_project_is_refused() {
    let layout = layout();
    let bare = layout.outer.join("bare");
    for command in LOADS_THE_BUNDLE {
        assert!(!approved_in(&bare, command), "`{command}` loaded the Gemfile above the project");
        let line = format!("cd src && {command}");
        assert!(!approved_in(&bare, &line), "`{line}` loaded the Gemfile above the project");
    }
}

#[test]
fn the_projects_own_gemfile_is_loaded_from_any_folder_inside_it() {
    let layout = layout();
    let ruby = layout.outer.join("ruby");
    for command in LOADS_THE_BUNDLE {
        assert!(approved_in(&ruby, command), "`{command}` should stay approved");
        let line = format!("cd src && {command}");
        assert!(approved_in(&ruby, &line), "`{line}` should stay approved");
    }
}

#[test]
fn bundle_exec_runs_the_folder_code_of_a_folder_a_cd_moved_to() {
    let layout = layout();
    let ruby = layout.outer.join("ruby");
    for inner in ["gem --version", "rake --version", "rspec --version", "rubocop --version", "rails --version", "rdoc --version"] {
        for folder in ["/tmp/x", "/Users/Shared/evil", ".."] {
            let line = format!("cd {folder} && bundle exec {inner}");
            assert!(!approved_in(&ruby, &line), "`{line}` was approved");
        }
    }
}
