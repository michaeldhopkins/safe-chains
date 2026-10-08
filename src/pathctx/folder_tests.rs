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
        crate::cst::command_verdict("mise trust")
    };
    assert!(writes(known), "precondition: `mise trust` writes in a known folder");
    let _fresh = enter(FolderLevel::Workspace);
    assert!(!crate::cst::command_verdict("mise trust").is_allowed());
    assert!(
        notes()
            .iter()
            .any(|n| matches!(n, Note::Leaf { command, admitted: false, .. } if command.starts_with("mise trust"))),
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
