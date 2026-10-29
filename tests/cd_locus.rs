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

/// A frozen folder (`.git`, `.jj`, `.hg`, `.sl`) or an installed package folder holds files no
/// commit of the project shows, so a tool that runs the code of the folder it runs in is refused
/// there, however the name is spelled. A case or Unicode variant names the same folder only where the
/// file system folds names, as macOS does.
#[test]
fn a_tool_that_runs_the_folders_code_is_refused_in_a_frozen_folder() {
    let mut folders = vec![".git", ".jj", ".hg", ".sl", ".git/hooks", "node_modules/x", "src/.git"];
    if cfg!(target_os = "macos") {
        folders.extend([".GIT", ".ſl"]);
    }
    for command in ["make -n", "bundle list", "nox -l", "mvn validate", "task -l", "gradle tasks", "bundle exec rake -T"] {
        for folder in &folders {
            let line = format!("cd {folder} && {command}");
            assert!(!approved(&line), "`{line}` was approved");
        }
    }
    for command in ["make -n -C .git", "make -n -C .jj", "make -nC.git", "make -n --directory=.git", "make -n -C node_modules/x"] {
        assert!(!approved(command), "`{command}` was approved");
    }
    for command in ["cd .git && ls", "cd .git && git log", "cd .git && cat HEAD", "cd src && make -n"] {
        assert!(approved(command), "`{command}` should stay approved");
    }
}

/// git obeys the configuration of the repository it runs in (`core.fsmonitor` and `core.pager` run
/// programs), so a repository outside the project is not one whose configuration anyone reviewed.
#[test]
fn git_in_a_repository_outside_the_project_is_refused() {
    for command in [
        "git -C /Users/Shared/repo status", "git -C /tmp/x/repo log", "cd /tmp/x/repo && git status",
        "cd /Users/Shared/repo && git log -p", "cd src && git -C .. -C ../grepo status", "git -C .. status",
        "git -C src -C ../.. diff --stat", "git -C ~/projects/other log -p", "git -C $HOME status", "cd / && git status",
        "pushd /tmp/x && git status", "git --git-dir=/tmp/x/.git -C /tmp/x log",
    ] {
        assert!(!approved(command), "`{command}` was approved");
    }
    for command in [
        "git status", "git log", "cd src && git status", "git -C src status", "git -C src -C .. log", "cd .git && git log",
        "git -C \"$PWD\" log --oneline", "cd /tmp/x && git --version", "git -C /tmp/x --version",
    ] {
        assert!(approved(command), "`{command}` should stay approved");
    }
}

/// The tools that open the repository they run in obey its configuration as git does: branchdiff and
/// tig run git there (`core.fsmonitor`), hg runs the hooks in `.hg/hgrc`, git-lfs and gitleaks call
/// git in it. A repository outside the project is refused for each, however the tool is pointed at it.
#[test]
fn a_tool_that_opens_a_repository_outside_the_project_is_refused() {
    for command in [
        "cd /tmp/x/repo && branchdiff -p --no-auto-fetch",
        "branchdiff -p /tmp/x/repo",
        "branchdiff ~/projects/other",
        "branchdiff -p ..",
        "cd /tmp/x/repo && tig",
        "tig -C /tmp/x/repo",
        "cd /tmp/x/repo && tig log",
        "cd /tmp/x/repo && hg status",
        "cd /Users/Shared/repo && hg log",
        "cd /tmp/x/repo && git-lfs ls-files",
        "cd /tmp/x/repo && git-lfs status",
        "cd /tmp/x/repo && gitleaks detect",
        "gitleaks detect -s /tmp/x/repo",
        "gitleaks detect --source=/tmp/x/repo",
        "gitleaks detect --source /tmp/x/repo",
        "cd src && gitleaks detect -s ../..",
        "tig -C. -C../jjt",
        "tig -Csrc -C../..",
        "tig -C. -C/tmp/x/repo",
        "gitleaks detect -s . -s ../jjt",
        "gitleaks detect -s. -s../jjt",
        "gitleaks detect --source=. --source=../jjt",
        "gitleaks protect -s . -s ../jjt",
    ] {
        assert!(!approved(command), "`{command}` was approved");
    }
    for command in [
        "branchdiff -p --no-auto-fetch", "branchdiff -p", "branchdiff -p .", "cd src && branchdiff -p", "tig", "cd src && tig",
        "git-lfs ls-files", "gitleaks detect", "gitleaks detect -s src", "cd /tmp/x && tig --version", "cd /tmp/x && gitleaks version",
        "tig -Csrc", "gitleaks detect -s src -s .",
    ] {
        assert!(approved(command), "`{command}` should stay approved");
    }
}

proptest::proptest! {
    /// Every way of pointing a repository-opening tool at a folder outside the project is refused,
    /// wherever that folder is.
    #[test]
    fn a_repository_opener_refuses_every_folder_outside_the_project(
        tool in proptest::sample::select(vec!["branchdiff -p", "tig", "hg status", "git-lfs ls-files", "gitleaks detect"]),
        dir in "/(tmp|Users/Shared|opt|var)/[a-z]{1,6}",
        spelling in proptest::sample::select(vec!["cd {d} && {t}", "pushd {d} && {t}", "(cd {d}; {t})", "cd src && cd {d} && {t}"]),
    ) {
        let line = spelling.replace("{d}", &dir).replace("{t}", tool);
        proptest::prop_assert!(!approved(&line), "`{}` was approved", line);
    }

    /// A tool given its repository flag more than once is judged by every one of them: tig moves
    /// through each `-C` in turn, and gitleaks scans every source it is given.
    #[test]
    fn a_repeated_repository_flag_is_judged_in_full(
        (tool, flag) in proptest::sample::select(vec![
            ("tig", "-C{d}"), ("tig", "-C {d}"),
            ("gitleaks detect", "-s {d}"), ("gitleaks protect", "-s{d}"),
            ("gitleaks detect", "--source={d}"), ("gitleaks detect", "--source {d}"),
        ]),
        inside in proptest::sample::select(vec![".", "src", "tests"]),
        outside in proptest::sample::select(vec!["../../jjt", "/tmp/x/repo", "../../..", "~/other"]),
        outside_first in proptest::bool::ANY,
    ) {
        let (first, second) = if outside_first { (outside, inside) } else { (inside, outside) };
        let line = format!("{tool} {} {}", flag.replace("{d}", first), flag.replace("{d}", second));
        proptest::prop_assert!(!approved(&line), "`{}` was approved", line);
    }
}
