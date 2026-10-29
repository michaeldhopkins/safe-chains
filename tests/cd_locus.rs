//! A `cd` earlier on the line moves every write that follows it, not only the writes that name a
//! path. A tool that writes the folder it runs in (`cargo fmt`, `git commit`, `mise set`) was judged
//! against the folder the harness reported, so `cd ~ && mise set X=1` wrote `~/mise.toml` unasked
//! while `cd ~ && touch x` asked. The registry-wide property is
//! `pathctx::folder::tests::a_declared_folder_write_follows_the_working_directory`; this file probes
//! the reported lines with the real project as `--cwd` and `--root`.

use safe_chains::pathctx::{PathCtx, enter};

fn approved(command: &str) -> bool {
    let root = env!("CARGO_MANIFEST_DIR").to_string();
    let _ctx = enter(PathCtx { cwd: Some(root.clone()), root: Some(root), session_id: None });
    safe_chains::is_safe_command(command)
}

#[test]
fn the_reported_lines_ask() {
    for command in [
        "cd ~ && mise set X=1",
        "cd ~/.config/mise && mise set PROMPT_COMMAND=id",
        "cd ~ && cargo fmt",
        "cd / && git commit -m x",
        "cd ~ && touch x",
        "cd - && cargo fmt",
        "pushd ~ && cargo fmt",
        "cd ~ && cargo build",
        "(cd ~ && cargo fmt)",
        "cd /tmp/x && cargo run",
        "cd /tmp/x && cargo build",
        "cd /tmp/x && cargo test",
        "cd ~/other && cargo check",
        "cd /private/tmp/x && cargo clippy",
    ] {
        assert!(!approved(command), "`{command}` was approved");
    }
}

#[test]
fn a_cd_that_stays_inside_or_ends_with_its_subshell_keeps_the_verdict() {
    for command in ["cargo fmt", "git commit -m x", "mise set X=1", "cargo build", "touch x"] {
        assert!(approved(command), "precondition: `{command}`");
        for line in [format!("cd src && {command}"), format!("cd ./src && {command}"), format!("(cd ~); {command}")] {
            assert!(approved(&line), "`{line}` should stay approved");
        }
    }
}

#[test]
fn what_an_if_condition_leaves_in_the_shell_reaches_its_branches() {
    for command in [
        "if cd ~; then cargo fmt; fi",
        "if cd ~; then true; else cargo fmt; fi",
        "if false; then true; elif cd ~; then cargo fmt; fi",
        "if cd ~; then true; elif cargo fmt; then true; fi",
        "X=./a; if X=~/.ssh/id_rsa; then cat \"$X\"; fi",
    ] {
        assert!(!approved(command), "`{command}` was approved");
    }
    for command in ["if cargo fmt; then cargo fmt; fi", "if test -d src; then cargo fmt; else cargo fmt; fi"] {
        assert!(approved(command), "`{command}` should stay approved");
    }
}

/// A tool that runs code the folder holds (a pom's plugins, a Taskfile, a dynamic inventory, a
/// mise.toml's tasks and hooks) runs the code of the folder a `cd` moved to.
#[test]
fn a_tool_that_runs_the_folders_code_follows_a_cd() {
    for command in [
        "mvn test", "mvn verify", "mvn validate", "mvn dependency:tree", "task -l", "task --list", "ansible-inventory --list",
        "ansible-playbook --syntax-check site.yml", "ansible-playbook --list-tasks site.yml", "mise exec -- ls", "mise x -- ls",
        "mise install", "mise upgrade", "mise run build",
    ] {
        for line in [format!("cd /Users/Shared/evil && {command}"), format!("cd /tmp/x && {command}")] {
            assert!(!approved(&line), "`{line}` was approved");
        }
    }
}

#[test]
fn a_tool_that_runs_the_folders_code_keeps_its_verdict_inside_the_project() {
    for command in ["mvn test", "mvn --version", "task -l", "ansible-inventory --list", "mise install", "mise exec -- ls"] {
        assert!(approved(command), "precondition: `{command}`");
        let line = format!("cd src && {command}");
        assert!(approved(&line), "`{line}` should stay approved");
    }
    assert!(approved("cd /tmp/x && mvn --version"), "the version query runs no folder code");
}
