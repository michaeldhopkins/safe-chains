use super::*;
use crate::pathctx::PathCtx;

const ROOT: &str = "/work/projects/app";

fn verdict_at(level: Option<FolderLevel>, command: &str) -> Verdict {
    let _ctx = crate::pathctx::enter(PathCtx {
        cwd: Some(crate::targets::UNKNOWN_WORKDIR.to_string()),
        root: Some(ROOT.to_string()),
        session_id: None,
    });
    let _folder = level.map(enter);
    crate::cst::command_verdict(command)
}

fn writes(v: Verdict) -> bool {
    matches!(v, Verdict::Allowed(l) if l > SafetyLevel::SafeRead)
}

#[test]
fn the_unknown_folder_and_below_it_are_unknown_and_nothing_else_is() {
    let base = crate::targets::UNKNOWN_WORKDIR;
    assert!(is_unknown(base));
    assert!(is_unknown(&format!("{base}/sub")));
    assert!(!is_unknown(&format!("{base}-evil")));
    assert!(!is_unknown("/nonexistent"));
    assert!(!is_unknown(ROOT));
}

#[test]
fn developer_approves_plain_writes_and_declared_writers() {
    for command in ["echo x > out.txt", "mkdir -p build/x", "touch notes.md", "cargo build", "cargo fmt", "git commit -am x", "git add ."] {
        assert!(writes(verdict_at(Some(FolderLevel::Developer), command)), "`{command}` should be approved at developer");
        assert!(!writes(verdict_at(None, command)) || verdict_at(None, command).is_allowed(), "precondition");
    }
}

#[test]
fn developer_leaves_sensitive_climbing_and_deleting_writes_to_the_prompt() {
    for command in
        ["echo x >> .zshrc", "tee .git/hooks/pre-commit", "echo x > ../x", "rm -rf build", "rm out.txt", "mv a b", "cp x authorized_keys"]
    {
        assert!(!verdict_at(Some(FolderLevel::Developer), command).is_allowed(), "`{command}` must not be approved at developer");
    }
}

#[test]
fn workspace_adds_deletion_and_a_sibling_but_never_a_sensitive_name() {
    let w = Some(FolderLevel::Workspace);
    assert!(verdict_at(w, "rm -rf build").is_allowed());
    assert!(verdict_at(w, "rm out.txt").is_allowed());
    for command in ["echo x >> .zshrc", "rm -rf .", "rm -rf .git", "echo x > ../../x", "tee hooks/pre-commit"] {
        assert!(!verdict_at(w, command).is_allowed(), "`{command}` must not be approved at workspace");
    }
}

#[test]
fn an_unlabelled_writer_fails_closed_and_is_recorded() {
    let _ctx = crate::pathctx::enter(PathCtx {
        cwd: Some(crate::targets::UNKNOWN_WORKDIR.to_string()),
        root: Some(ROOT.to_string()),
        session_id: None,
    });
    let _folder = enter(FolderLevel::Workspace);
    let known = {
        let _known = crate::pathctx::enter_cwd(Some(ROOT.to_string()));
        crate::cst::command_verdict("mise untrust")
    };
    assert!(writes(known), "precondition: `mise untrust` writes in a known folder");
    let _fresh = enter(FolderLevel::Workspace);
    assert!(!crate::cst::command_verdict("mise untrust").is_allowed());
    assert!(
        notes()
            .iter()
            .any(|n| matches!(n, Note::Leaf { command, admitted: false, .. } if command.starts_with("mise untrust"))),
        "{:?}",
        notes()
    );
}

#[test]
fn a_cd_to_a_known_place_is_judged_as_a_known_folder() {
    assert_eq!(verdict_at(Some(FolderLevel::Developer), "cd /tmp && rm -rf x"), verdict_at(None, "cd /tmp && rm -rf x"));
    assert!(
        !verdict_at(Some(FolderLevel::Developer), "cd .ssh && echo x > config").is_allowed(),
        "a cd below the unknown folder keeps its path"
    );
}

#[test]
fn outside_the_mode_and_at_reads_nothing_changes() {
    for command in ["echo x > out.txt", "cargo build", "rm -rf build", "echo x >> .zshrc", "cat notes.md", "git status"] {
        assert_eq!(verdict_at(None, command), verdict_at(Some(FolderLevel::Reads), command), "{command}");
    }
}

#[test]
fn a_frame_is_popped_even_when_classification_unwinds() {
    let _folder = enter(FolderLevel::Developer);
    let tokens = vec![Token::from_raw("x".to_string())];
    let caught = std::panic::catch_unwind(|| judge_leaf(&tokens, false, || panic!("boom")));
    assert!(caught.is_err());
    assert!(FRAMES.with(|f| f.borrow().is_empty()));
}

fn verdict_in(cwd: &str, command: &str) -> Verdict {
    let _ctx = crate::pathctx::enter(PathCtx { cwd: Some(cwd.to_string()), root: Some(ROOT.to_string()), session_id: None });
    crate::cst::command_verdict(command)
}

fn outside_the_workspace() -> Vec<String> {
    let home = std::env::var("HOME").expect("HOME");
    vec![home.clone(), "/".to_string(), format!("{home}/.config/mise"), "/work/projects/other".to_string()]
}

/// Every sub the registry declares as writing, or running code from, the folder it runs in: approved
/// in the workspace, it must ask once a `cd` has taken it out, or when the harness says it already
/// runs out there, and keep its verdict after a `cd` that stays inside.
#[test]
fn a_declared_folder_write_follows_the_working_directory() {
    let mut checked = 0;
    let mut wrong = Vec::new();
    for (command, anchor) in crate::registry::declared_writers() {
        if !matches!(anchor, Anchor::ImplicitSource | Anchor::ImplicitOutput | Anchor::RunsFolderCode) {
            continue;
        }
        let at_root = verdict_in(ROOT, &command);
        if !writes(at_root) {
            continue;
        }
        checked += 1;
        for dir in outside_the_workspace() {
            for (cwd, line) in [(ROOT.to_string(), format!("cd {dir} && {command}")), (dir.clone(), command.clone())] {
                if verdict_in(&cwd, &line).is_allowed() {
                    wrong.push(format!("`{line}` in {cwd}"));
                }
            }
        }
        for line in [format!("cd src && {command}"), format!("(cd /; true) && {command}")] {
            if verdict_in(ROOT, &line) != at_root {
                wrong.push(format!("`{line}` changed its verdict inside the workspace"));
            }
        }
    }
    assert!(checked > 100, "only {checked} declared folder writers are approved at the root");
    assert!(wrong.is_empty(), "{} wrong, first:\n{}", wrong.len(), wrong.iter().take(25).cloned().collect::<Vec<_>>().join("\n"));
}

/// A `cd` whose target the classifier cannot name leaves the folder unknown, so a folder write after
/// it asks.
#[test]
fn a_folder_write_after_an_unfollowable_cd_asks() {
    for cd in ["cd -", "cd", "cd \"$UNSET_DIR\"", "{ cd ~; }", "if true; then cd /; fi", "popd", "pushd ~"] {
        for command in ["cargo fmt", "git commit -m x", "mise set X=1", "cargo build"] {
            assert!(writes(verdict_in(ROOT, command)), "precondition: `{command}`");
            let line = format!("{cd}; {command}");
            assert!(!verdict_in(ROOT, &line).is_allowed(), "`{line}` was approved");
        }
    }
}

#[test]
fn a_named_write_and_a_fixed_target_do_not_take_the_folder() {
    let home = std::env::var("HOME").expect("HOME");
    for command in ["touch /tmp/x", "echo x > /tmp/y", "ls", "git --version"] {
        let line = format!("cd {home} && {command}");
        assert_eq!(verdict_in(ROOT, &line), verdict_in(&home, command), "{line}");
        assert!(verdict_in(ROOT, &line).is_allowed(), "{line}");
    }
}

/// Every sub the registry declares as running the folder's code, or building into it (which runs
/// build scripts): approved in the workspace, it must ask once a `cd` has taken it to a folder whose
/// code is not the project's, a temporary one included, where a plain write would be approved.
#[test]
fn a_declared_folder_code_runner_follows_the_working_directory_to_a_temporary_folder() {
    let mut checked = 0;
    let mut wrong = Vec::new();
    for (command, anchor) in crate::registry::declared_writers() {
        if !matches!(anchor, Anchor::ImplicitOutput | Anchor::RunsFolderCode) {
            continue;
        }
        let at_root = verdict_in(ROOT, &command);
        if !at_root.is_allowed() {
            continue;
        }
        checked += 1;
        for dir in ["/tmp/x", "/private/tmp/x", "/var/folders/ab/T/x"] {
            let line = format!("cd {dir} && {command}");
            if verdict_in(ROOT, &line).is_allowed() {
                wrong.push(format!("`{line}`"));
            }
        }
        let line = format!("cd src && {command}");
        if verdict_in(ROOT, &line) != at_root {
            wrong.push(format!("`{line}` changed its verdict inside the workspace"));
        }
    }
    assert!(checked > 50, "only {checked} declared folder code runners are approved at the root");
    assert!(wrong.is_empty(), "{} wrong, first:\n{}", wrong.len(), wrong.iter().take(25).cloned().collect::<Vec<_>>().join("\n"));
}
