//! A tool that takes code or configuration from the folder it runs in (`folder_code` in
//! `pathgates.toml`) is judged by that folder's origin, the way a script there would be.
//!
//! `cargo --version` from a folder holding `rust-toolchain.toml` with `path = "./tc"` runs
//! `./tc/bin/cargo`; `cargo build` there reads `.cargo/config.toml`, whose `rustc-wrapper` runs; and
//! `npm test` runs the folder's `package.json`. From a folder outside this project whose code
//! would not be executed — a temporary directory, a system directory, or another user's folder —
//! that is foreign code, so these ask just as `sh ./x.sh` there does. The project root, any folder
//! inside it, and a folder the user granted are unaffected.

#[derive(serde::Deserialize)]
struct Lists {
    folder_code: std::collections::HashSet<String>,
}

pub(super) static NAMES: std::sync::LazyLock<std::collections::HashSet<String>> = std::sync::LazyLock::new(|| {
    toml::from_str::<Lists>(include_str!("../../pathgates.toml"))
        .expect("pathgates.toml is invalid TOML")
        .folder_code
});

/// Whether `cmd` is a `folder_code` tool and the folder it would run in is foreign.
pub(super) fn denies(cmd: &str) -> bool {
    NAMES.contains(cmd) && is_foreign()
}

/// A read or an inert command in the unknown folder at `reads`. The ceiling would approve it,
/// so it asks here. A write stays allowed: that ceiling is what sends it to the prompt, and it
/// says why.
pub(super) fn denies_read(cmd: &str, tokens: &[crate::parse::Token]) -> bool {
    if tokens.is_empty() || !NAMES.contains(cmd) || !unknown_at_reads() || READING_UNSCOPED.with(std::cell::Cell::get) {
        return false;
    }
    let verdict = unscoped_verdict(tokens);
    matches!(verdict, crate::verdict::Verdict::Allowed(level) if level <= crate::verdict::SafetyLevel::SafeRead)
}

fn unknown_at_reads() -> bool {
    let Some(cwd) = crate::pathctx::cwd() else { return false };
    crate::pathctx::folder::is_unknown(&cwd) && crate::pathctx::folder::level() == Some(crate::pathctx::anchor::FolderLevel::Reads)
}

std::thread_local! {
    static READING_UNSCOPED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The verdict before this read check. The engine's own verdict does not consult the gate; the
/// legacy one does, so a second look skips this check and cannot recurse.
fn unscoped_verdict(tokens: &[crate::parse::Token]) -> crate::verdict::Verdict {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            READING_UNSCOPED.with(|cell| cell.set(self.0));
        }
    }
    let previous = READING_UNSCOPED.with(|cell| cell.replace(true));
    let _restore = Restore(previous);
    crate::engine::bridge::engine_verdict(tokens).unwrap_or_else(|| crate::handlers::dispatch(tokens))
}

/// Whether the ambient folder's code would not be run. A directory inside the physically resolved
/// project root is the project. Anywhere else is foreign when executing a file there would be
/// refused, and a temporary directory is foreign even when its spelling is not the exact temp
/// prefix, except this session's scratchpad. A `cd` to a place nobody can pin counts as foreign.
fn is_foreign() -> bool {
    let Some(cwd) = crate::pathctx::cwd() else { return false };
    if cwd == crate::pathctx::UNRESOLVED_CWD {
        return true;
    }
    // The harness places a command here when it does not know the folder. With no level in
    // force, a tool that runs the folder's own code asks. Developer and workspace treat the
    // sentinel as that folder. At reads a write stays a write, so the ceiling can explain it.
    if crate::pathctx::folder::is_unknown(&cwd) {
        return crate::pathctx::folder::level().is_none();
    }
    if !cwd.starts_with('/') {
        return false;
    }
    let cwd = crate::pathctx::physical_directory(&cwd);
    if cwd == crate::pathctx::UNRESOLVED_CWD {
        return true;
    }
    let root = crate::pathctx::root().map(|root| crate::pathctx::physical_directory(&root));
    if root.as_deref().is_some_and(|root| inside(&cwd, root)) {
        return false;
    }
    let dir = format!("{}/", cwd.trim_end_matches('/'));
    if crate::engine::resolve::locus::under_temp(&dir) && !crate::pathctx::in_session_scratchpad(&cwd) {
        return true;
    }
    !crate::engine::resolve::execute_file_verdict(&cwd).is_allowed()
}

fn inside(path: &str, root: &str) -> bool {
    let root = root.trim_end_matches('/');
    let path = path.trim_end_matches('/');
    !root.is_empty() && (path == root || path.strip_prefix(root).is_some_and(|rest| rest.starts_with('/')))
}

#[cfg(test)]
mod tests {
    use crate::pathctx::{PathCtx, enter};
    use proptest::prelude::*;

    const SESSION: &str = "sc0000folder0000code";
    const PROBES: &[&str] = &["--version", "-V", "version", "--help", "help", "-v"];

    fn project() -> String {
        format!("{}/projects/scproj", std::env::var("HOME").expect("HOME"))
    }

    fn judged_from(cwd: &str, command: &str) -> bool {
        let root = project();
        let _g = enter(PathCtx { cwd: Some(cwd.to_string()), root: Some(root), session_id: Some(SESSION.to_string()) });
        crate::is_safe_command(command)
    }

    fn approved_probe(name: &str) -> Option<String> {
        PROBES.iter().map(|p| format!("{name} {p}")).find(|c| judged_from(&project(), c))
    }

    fn known_names() -> Vec<String> {
        let mut names: Vec<String> = crate::handlers::handler_docs()
            .into_iter()
            .flat_map(|d| d.name.split(" / ").map(str::to_string).collect::<Vec<_>>())
            .collect();
        names.extend(crate::registry::toml_command_names().into_iter().map(str::to_string));
        names
    }

    #[test]
    fn every_folder_code_name_is_a_command_the_classifier_knows() {
        let known = known_names();
        for name in super::super::folder_code_names() {
            assert!(known.iter().any(|k| k == name), "folder_code lists `{name}`, which no handler or TOML entry defines");
        }
    }

    #[test]
    fn the_reported_bypasses_ask_from_a_foreign_temp_folder() {
        let root = project();
        for command in [
            "cd /tmp/x && cargo --version",
            "cd /tmp/x && rustc --version",
            "cd /tmp/x && rustup show",
            &format!("cd /tmp/y && cargo check --manifest-path {root}/Cargo.toml"),
            "cd /tmp/y && cargo build",
            "cd /tmp/y && cargo test",
            "cd /tmp/y && npm test",
            "cd /tmp/y && go test ./...",
            "cd /private/tmp/y && cargo build",
            "cd /var/tmp/y && cargo build",
            "pushd /tmp/y && cargo build",
            "cd /tmp/y; cargo build",
            "cd /tmp/y && env cargo build",
            "cd /tmp/y && timeout 60 cargo build",
            "cd /tmp/y && rustup run stable cargo build",
        ] {
            assert!(!judged_from(&root, command), "{command}");
        }
        for command in ["cargo --version", "cargo build", "npm test", "go test ./..."] {
            assert!(!judged_from("/tmp/y", command), "ambient /tmp/y: {command}");
            assert!(judged_from(&root, command), "{command}");
            assert!(judged_from(&format!("{root}/sub"), command), "below the root: {command}");
            assert!(judged_from(&format!("/private/tmp/claude-501/x/{SESSION}/scratchpad"), command), "scratchpad: {command}");
        }
        assert!(judged_from(&root, "cd /tmp/y && cat notes.txt"), "a folder in temp is still readable");
    }

    #[test]
    fn a_folder_code_tool_asks_from_a_folder_outside_the_project() {
        let root = project();
        let outside = [
            "cd /etc && cargo test",
            "cd /etc && pytest",
            "cd /TMP/y && cargo build",
            "cd /Tmp/y && cargo build",
            "cd /PRIVATE/TMP/y && cargo build",
            "cd /private/TMP/y && cargo build",
            "cd /System/Volumes/Data/tmp/y && cargo build",
            "cd /System/Volumes/Data/private/tmp/y && cargo build",
            "cd /private/var/folders/xx/yy/T/y && cargo build",
            "cd /var/folders/xx/yy/T/y && cargo build",
            "cd /VAR/folders/xx/yy/T/y && cargo build",
            "cd /Var/Folders/xx/yy/T/y && cargo build",
            "cd ~/Downloads && cargo test",
            "cd /Users/Shared/evil && pytest",
            "cd /opt/y && npm test",
        ];
        let still: Vec<_> = outside.into_iter().filter(|command| judged_from(&root, command)).collect();
        assert!(still.is_empty(), "still approved outside the project: {still:?}");
        for command in ["cargo build", "cargo test", "npm test", "pytest", "rustup show", "jest", "eslint src/"] {
            assert!(judged_from(&root, command), "{command}");
            assert!(judged_from(&root, &format!("cd sub && {command}")), "cd sub && {command}");
        }
        assert!(judged_from(&root, "cd /tmp/y && cat notes.txt"));
    }

    #[test]
    fn a_sibling_runner_asks_outside_the_project() {
        let root = project();
        for command in [
            "cd /tmp/y && jasmine", "cd /tmp/y && cucumber-js", "cd /tmp/y && gotestsum", "cd /tmp/y && karma start --single-run",
            "cd /tmp/y && turbo run build --dry-run", "cd /tmp/y && artisan test", "cd /tmp/y && please list",
        ] {
            assert!(!judged_from(&root, command), "{command}");
        }
        for command in
            ["jasmine", "cucumber-js", "gotestsum", "karma start --single-run", "turbo run build --dry-run", "artisan test", "please list"]
        {
            assert!(judged_from(&root, command), "{command}");
        }
    }

    /// The unknown-folder sentinel is not a directory the harness has pinned. A folder-code tool
    /// asks there until a level that approves writes in that folder is in force. At `reads` a
    /// write stays allowed in the library, because that ceiling sends it to the prompt and says
    /// why; a read asks here, because the ceiling would approve it.
    #[test]
    fn a_folder_code_tool_asks_in_the_unknown_folder_until_a_write_level() {
        use crate::pathctx::anchor::FolderLevel;
        use crate::pathctx::folder::enter as enter_folder;
        let root = project();
        let cwd = crate::targets::UNKNOWN_WORKDIR.to_string();
        let reads = ["pytest", "npm test", "cargo --version"];
        {
            let _unknown = enter(PathCtx { cwd: Some(cwd.clone()), root: Some(root.clone()), session_id: None });
            for command in reads {
                assert!(!crate::is_safe_command(command), "no level: {command}");
            }
            assert!(!crate::is_safe_command("cargo fmt"), "no level: cargo fmt");
            assert!(crate::is_safe_command("cat notes.txt"));
        }
        {
            let _unknown = enter(PathCtx { cwd: Some(cwd.clone()), root: Some(root.clone()), session_id: None });
            let _reads = enter_folder(FolderLevel::Reads);
            for command in reads {
                assert!(!crate::is_safe_command(command), "reads: {command}");
            }
            assert!(crate::is_safe_command("cargo fmt"), "reads: cargo fmt stays a write");
            assert!(crate::is_safe_command("cat notes.txt"));
        }
        {
            let _unknown = enter(PathCtx { cwd: Some(cwd.clone()), root: Some(root.clone()), session_id: None });
            let _developer = enter_folder(FolderLevel::Developer);
            for command in reads {
                assert!(crate::is_safe_command(command), "developer: {command}");
            }
            assert!(crate::is_safe_command("cargo fmt"), "developer: cargo fmt");
        }
        let _project = enter(PathCtx { cwd: Some(root.clone()), root: Some(root), session_id: None });
        assert!(!crate::is_safe_command(&format!("cd {cwd} && cargo fmt")));
        assert!(crate::is_safe_command(&format!("cd {cwd} && cat notes.txt")));
        assert!(!crate::is_safe_command("cd /tmp/y && cargo fmt"));
    }

    #[test]
    fn a_symlinked_temp_directory_is_foreign_for_folder_code() {
        let proj = tempfile::tempdir().expect("project dir");
        let root = proj.path().canonicalize().expect("project path").to_string_lossy().into_owned();
        let target = format!("/tmp/sc-fix-02413-{}", std::process::id());
        std::fs::create_dir_all(&target).expect("temp target");
        let _cleanup = RemoveDir(target.clone());
        std::os::unix::fs::symlink(&target, format!("{root}/tmp_link")).expect("tmp_link");
        let _g = enter(PathCtx { cwd: Some(root.clone()), root: Some(root), session_id: Some(SESSION.to_string()) });
        assert!(crate::is_safe_command("cargo build"), "the project itself stays the project");
        assert!(!crate::is_safe_command("cd ./tmp_link && cargo build"), "cd ./tmp_link && cargo build");
        assert!(!crate::is_safe_command("cd tmp_link && cargo test"), "cd tmp_link && cargo test");
    }

    struct RemoveDir(String);

    impl Drop for RemoveDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn cd_pwd_after_leaving_the_project_does_not_count_as_the_project() {
        let root = project();
        let outside = [
            "cd /tmp/y && cd $(pwd) && cargo build",
            "cd /tmp/y && cd \"$(pwd)\" && cargo build",
            "cd /tmp/y && cd `pwd` && cargo build",
            "cd /tmp/y && cd $(pwd) && cargo test",
            "cd /tmp/y && cd $(pwd) && pytest",
            "cd /TMP/y && cd $(pwd) && cargo build",
            "cd /etc && cd $(pwd) && cargo test",
            "cd /etc && cd $(pwd) && pytest",
            "cd /private/var/folders/xx/yy/T/y && cd $(pwd) && cargo build",
            "cd /System/Volumes/Data/tmp/y && cd $(pwd) && cargo build",
            "cd /tmp/y && cd $(pwd)/src && cargo build",
            "cd /tmp/y && cd $(pwd) && sh ./x.sh",
            "cd /tmp/y && cd $(pwd) && bash ./x.sh",
            "cd /tmp/y && cd $(pwd) && python3 ./x.py",
            "cd /tmp/y && cd $(pwd) && node ./x.js",
        ];
        let still: Vec<_> = outside.into_iter().filter(|command| judged_from(&root, command)).collect();
        assert!(still.is_empty(), "still approved: {still:?}");
        for command in [
            "cd $(pwd) && cargo build", "cd $(pwd) && cargo test", "cd $(pwd) && cat f", "cd $(pwd) && npm test", "cd $(pwd) && pytest",
            "cd $(pwd) && rustup show", "cd $(pwd) && jest", "cd \"$(pwd)\" && eslint src/", "cd sub && cd $(pwd) && cargo build",
            "cd ./sub && cargo test", "cd /tmp/y && cat notes.txt", "cd /tmp/y && cd $(pwd) && cat notes.txt", "sh ./x.sh",
            "cd $(pwd) && sh ./x.sh", "cd $(pwd) && bash ./x.sh", "cd $(pwd) && node ./x.js", "cargo build", "cargo test", "npm test",
            "pytest", "rustup show", "jest", "eslint src/",
        ] {
            assert!(judged_from(&root, command), "{command}");
        }
    }

    #[test]
    fn a_folder_code_tool_asks_where_the_folder_it_runs_in_is_unknown() {
        let root = project();
        for command in [
            "CDPATH=/tmp; cd y && cargo --version", "CDPATH=/tmp cd y && cargo build", "export CDPATH=/tmp; cd y && npm test",
            "cdpath=/tmp; cd y && go test ./...",
        ] {
            assert!(!judged_from(&root, command), "{command}");
        }
        for command in ["CDPATH=/tmp; cd ./sub && cargo build", "cd sub && cargo build", "cargo build"] {
            assert!(judged_from(&root, command), "{command}");
        }
    }

    #[test]
    fn tools_that_run_their_folders_config_or_tests_ask_from_a_foreign_temp_folder() {
        for command in [
            "vite build", "webpack", "rollup -c", "tsup", "mocha", "ava", "stylelint a.css", "pylint x.py", "flake8", "phpunit",
            "phpstan analyse", "rubocop", "mypy .", "next build", "astro build", "coverage run -m pytest", "cucumber", "php artisan test",
            "pod install", "tuist build", "mvn test", "sphinx-build docs out", "mkdocs build", "ansible-lint",
        ] {
            assert!(judged_from(&project(), command), "in the project: {command}");
            assert!(!judged_from("/tmp/y", command), "from /tmp/y: {command}");
        }
    }

    #[test]
    fn a_project_root_under_temp_is_still_the_project() {
        let root = "/tmp/ci/work/proj";
        let _g = enter(PathCtx { cwd: Some(root.to_string()), root: Some(root.to_string()), session_id: None });
        assert!(crate::is_safe_command("cargo build"));
        assert!(crate::is_safe_command("cd sub && cargo build"));
        assert!(!crate::is_safe_command("cd /tmp/other && cargo build"));
    }

    proptest! {
        #[test]
        fn a_folder_code_tool_asks_from_a_directory_outside_the_project(
            place in prop::sample::select(vec![
                "/etc", "/TMP/y", "/Tmp/z", "/PRIVATE/TMP/y", "/System/Volumes/Data/tmp/y",
                "/private/var/folders/xx/yy/T/y", "/VAR/folders/aa/bb/T/y", "/opt/y", "/Users/Shared/y",
            ]),
        ) {
            let root = project();
            let cargo_test = format!("cd {place} && cargo test");
            let pytest_cmd = format!("cd {place} && pytest");
            prop_assert!(!judged_from(&root, &cargo_test));
            prop_assert!(!judged_from(&root, &pytest_cmd));
            prop_assert!(judged_from(&root, "cargo build"));
            prop_assert!(judged_from(&root, "cargo test"));
            prop_assert!(judged_from(&root, "cd sub && cargo test"));
            prop_assert!(judged_from(&root, "npm test"));
            prop_assert!(judged_from(&root, "pytest"));
            prop_assert!(judged_from(&root, "rustup show"));
            prop_assert!(judged_from(&root, "jest"));
            prop_assert!(judged_from(&root, "eslint src/"));
        }

        #[test]
        fn a_folder_code_tool_asks_from_a_foreign_temp_folder_and_not_from_the_project(
            pick in any::<prop::sample::Index>(),
            temp in prop::sample::select(vec!["/tmp", "/private/tmp", "/var/tmp"]),
            dir in "[a-z]{1,8}",
        ) {
            let names = super::super::folder_code_names();
            let name = pick.get(&names);
            if let Some(probe) = approved_probe(name) {
                let foreign = format!("{temp}/{dir}");
                prop_assert!(!judged_from(&foreign, &probe), "ambient {foreign}: {probe}");
                prop_assert!(!judged_from(&project(), &format!("cd {foreign} && {probe}")), "cd {foreign} && {probe}");
                prop_assert!(judged_from(&format!("{}/{dir}", project()), &probe), "inside the project: {probe}");
            }
        }
    }

    #[test]
    fn most_folder_code_tools_have_an_approved_probe() {
        let names = super::super::folder_code_names();
        let probed = names.iter().filter(|n| approved_probe(n).is_some()).count();
        assert!(probed * 2 >= names.len(), "only {probed} of {} folder_code tools have an approved probe", names.len());
    }
}
