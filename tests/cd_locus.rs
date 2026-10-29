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

/// make parses the makefile, `$(shell …)` included, even under `-n`; Bundler evaluates the Gemfile
/// for every subcommand that reads the bundle; Gradle runs the build scripts for every task; nox
/// imports the noxfile to list its sessions. Each runs the code of the folder it is pointed at.
#[test]
fn a_build_tool_that_evaluates_the_folders_files_follows_a_cd_and_its_folder_flags() {
    for command in [
        "make -n", "make -n all", "make -nq", "make -p -n", "bundle list", "bundle check", "bundle show", "bundle info rake",
        "bundle install", "bundle exec rake -T", "gradle tasks", "gradle dependencies", "gradle properties", "gradle test", "gradle check",
        "gradle build", "nox -l", "nox --list",
    ] {
        for line in [format!("cd /Users/Shared/evil-sc && {command}"), format!("cd /tmp/x && {command}")] {
            assert!(!approved(&line), "`{line}` was approved");
        }
    }
    for command in [
        "make -n -C /Users/Shared/evil-sc", "make -C /x -n", "make -n -C /tmp/x", "make -n --directory=/tmp/x", "make -nC /tmp/x",
        "make -n -f /tmp/x/Makefile", "make -n --file=/tmp/x/Makefile", "make -n --makefile /tmp/x/Makefile", "make -n -f ../../Makefile",
        "bundle check --gemfile=/tmp/x/Gemfile", "bundle install --gemfile /tmp/x/Gemfile",
    ] {
        assert!(!approved(command), "`{command}` was approved");
    }
}

/// A make command-line assignment is evaluated before any rule (`A:=$(shell id)`), and a makefile
/// read from standard input is whatever the line pipes in, so neither is the project's makefile.
#[test]
fn a_make_assignment_or_a_makefile_from_stdin_is_refused() {
    for command in [
        "make -n 'A:=$(shell echo pwned >&2)'", "make -n 'A:=$(info $(OPENAI_API_KEY))'", "make -n -f /dev/null 'A:=$(info x)'",
        "make -n MAKEFLAGS=p", "make -n GNUMAKEFLAGS=p", "make -n A=1", "make -n -f /dev/stdin <<< '$(info x)'",
        "echo '$(info x)' | make -n -f -", "make -n -f-", "make -n --file=-", "make -n --makefile -", "make -n -f /dev/fd/0", "make -nf -",
    ] {
        assert!(!approved(command), "`{command}` was approved");
    }
}

#[test]
fn a_build_tool_that_evaluates_the_folders_files_keeps_its_verdict_inside_the_project() {
    for command in ["make -n", "make -n -C src", "make -n -f Makefile", "make -nq", "bundle list", "gradle tasks", "nox -l"] {
        assert!(approved(command), "precondition: `{command}`");
        let line = format!("cd src && {command}");
        assert!(approved(&line), "`{line}` should stay approved");
    }
    for command in ["cd /tmp/x && gradle --version", "cd /tmp/x && gradle --help"] {
        assert!(approved(command), "`{command}` runs no build script");
    }
}

/// make applies each `-C` in turn, each relative to the last, and reads a relative `-f` from the
/// folder the last one reached, so a line with more than one `-C`, or a `-C` and a relative `-f`,
/// names a folder no single operand shows.
#[test]
fn make_folders_given_more_than_once_are_refused() {
    for command in [
        "cd src && make -n -C .. -C ..", "make -n -C src -C ..", "make -n -C src -C .. -C ..", "cd src && make -n -C .. -f ../Makefile",
        "make -n -C src -f Makefile", "make -n --directory=src --directory=..", "cd src && make -nC .. -C ..", "make -nCsrc -C..",
        "make -n --directory src -C ..", "make -n -C src --file=../Makefile",
    ] {
        assert!(!approved(command), "`{command}` was approved");
    }
    for command in ["cd src && make -n -C ..", "make -n -C src", "make -nC src", "make -n -f Makefile", "make -n -C src all"] {
        assert!(approved(command), "`{command}` should stay approved");
    }
}

/// `-p` prints make's rule database and then builds as usual, so on its own it runs the recipes.
#[test]
fn make_print_database_alone_is_refused() {
    for command in ["make -p", "make --print-data-base", "make -p all", "make -pq"] {
        assert!(!approved(command), "`{command}` was approved");
    }
    for command in ["make -p -n", "make -pn", "make -n --print-data-base"] {
        assert!(approved(command), "`{command}` should stay approved");
    }
}
