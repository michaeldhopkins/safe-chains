//! A tool that takes code or configuration from the folder it runs in (`folder_code` in
//! `pathgates.toml`) is judged by that folder's origin, the way a script there would be.
//!
//! `cargo --version` from a folder holding `rust-toolchain.toml` with `path = "./tc"` runs
//! `./tc/bin/cargo`; `cargo build` there reads `.cargo/config.toml`, whose `rustc-wrapper` runs; and
//! `npm test` runs the folder's `package.json`. From a temporary directory that is not this
//! session's scratchpad, that is foreign code, so these ask just as `sh ./x.sh` there does. The
//! project root, any folder inside it, and a folder the user granted are unaffected.

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

/// Whether the ambient folder is a temporary directory whose code would not be run: outside the
/// project root, under a temp root, and refused at the executor locus (so neither this session's
/// scratchpad nor a grant). A `cd` the line made to a place nobody can pin (a CDPATH lookup, a
/// substitution) counts as foreign, since it may be one.
fn is_foreign() -> bool {
    let Some(cwd) = crate::pathctx::cwd() else { return false };
    if cwd == crate::pathctx::UNRESOLVED_CWD {
        return true;
    }
    if !cwd.starts_with('/') || !crate::pathctx::under_temp_root(&format!("{}/", cwd.trim_end_matches('/'))) {
        return false;
    }
    if crate::pathctx::root().is_some_and(|root| inside(&cwd, &root)) {
        return false;
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
