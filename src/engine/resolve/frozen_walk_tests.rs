use std::os::unix::fs::symlink;

use super::{Reach, Walk, reach_below};
use crate::pathctx::{self, PathCtx};
use crate::{command_verdict_in, facet_breakdown};
use proptest::prelude::*;

/// The phrase already used when a walk finds a frozen name, including when it stops at its entry bound
/// and stands that name in for the tree it could not finish.
const FROZEN_REASON: &str = "changes your project's .git, hooks or CI settings";

const GLOB_NAMES: &[&str] = &["*", "?", "a*", "b?", "[q]", "x*"];

fn project() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().expect("scratch");
    let root = dir.path().canonicalize().expect("canonical").to_str().expect("utf-8").to_string();
    (dir, root)
}

fn ctx(root: &str) -> PathCtx {
    PathCtx { cwd: Some(root.to_string()), root: Some(root.to_string()), session_id: None }
}

fn on_hook_stack<T: Send + 'static>(body: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(body)
        .expect("spawn")
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}

fn verdict_and_reason(root: &str, cmd: &str) -> (bool, String) {
    let allowed = command_verdict_in(cmd, ctx(root)).is_allowed();
    let why = {
        let _guard = pathctx::enter(ctx(root));
        facet_breakdown(cmd, false)
    };
    (allowed, why)
}

fn finished(reached: &[Reach]) -> bool {
    !reached.iter().any(|r| matches!(r, Reach::Unlisted(_)))
}

#[test]
fn an_unresolvable_symlink_cycle_is_not_auto_approved() {
    let (_dir, root) = project();
    symlink("loop", format!("{root}/loop")).expect("self loop");
    std::fs::create_dir(format!("{root}/pair")).expect("pair");
    symlink("b", format!("{root}/pair/a")).expect("a to b");
    symlink("a", format!("{root}/pair/b")).expect("b to a");
    let root_for_walk = root.clone();
    let (allowed, why) = on_hook_stack(move || verdict_and_reason(&root_for_walk, "cp -RL . backup"));
    assert!(!allowed, "symlink cycle was auto-approved: {why}");
}

#[test]
fn a_deep_directory_chain_is_walked_to_its_end() {
    let (_dir, root) = project();
    let mut at = root.clone();
    let mut depth = 0usize;
    while depth < 200 && at.len() + 2 < 900 {
        at.push_str("/d");
        std::fs::create_dir(&at).expect("deep directory");
        depth += 1;
    }
    assert!(depth >= 128, "built only {depth} levels at {at}");
    let rel = at.strip_prefix(&format!("{root}/")).expect("chain under root").to_string();
    let root_for_plain = root.clone();
    let (allowed, why) = on_hook_stack(move || verdict_and_reason(&root_for_plain, "chmod -R 755 ."));
    assert!(allowed, "deep tree of {depth} was refused after it was walked: {why}");
    std::fs::create_dir(format!("{at}/.git")).expect("git at the bottom");
    let root_for_git = root.clone();
    let (allowed, reached) = on_hook_stack(move || {
        let _guard = pathctx::enter(ctx(&root_for_git));
        let reached = reach_below(".", Walk::plain(true));
        let allowed = crate::command_verdict("chmod -R 755 .").is_allowed();
        (allowed, reached)
    });
    assert!(!allowed, "deep tree of {depth} hid .git and was auto-approved: {reached:?}");
    assert!(
        reached.iter().any(|r| matches!(r, Reach::Found(p) if p.contains(&format!("{rel}/.git/f")))),
        "walk did not reach the bottom of {rel}: {reached:?}"
    );
}

#[test]
fn a_link_back_to_its_parent_still_auto_approves() {
    let (_dir, root) = project();
    std::fs::create_dir(format!("{root}/sub")).expect("sub");
    symlink("..", format!("{root}/sub/up")).expect("up");
    std::fs::write(format!("{root}/sub/f.txt"), "x").expect("file");
    let root_for_walk = root.clone();
    let (allowed, why) = on_hook_stack(move || verdict_and_reason(&root_for_walk, "cp -RL sub backup"));
    assert!(allowed, "{why}");
}

#[test]
fn a_directory_whose_name_is_a_glob_is_listed_once() {
    let (_dir, root) = project();
    let star = format!("{root}/*");
    std::fs::create_dir(&star).expect("star directory");
    std::fs::create_dir(format!("{star}/.git")).expect("git");
    let also = format!("{root}/a*");
    std::fs::create_dir(&also).expect("a-star directory");
    std::fs::create_dir(format!("{also}/.git")).expect("git in a-star");
    let root_for_walk = root.clone();
    let (reached, allowed, why) = on_hook_stack(move || {
        let _guard = pathctx::enter(ctx(&root_for_walk));
        let star = reach_below("*", Walk::plain(true));
        let named = reach_below("a*", Walk::plain(true));
        let (allowed, why) = verdict_and_reason(&root_for_walk, "chmod -R 755 *");
        ((star, named), allowed, why)
    });
    assert!(matches!(reached.0.as_slice(), [Reach::Found(p)] if p.ends_with(".git/f")), "star walk: {:?}", reached.0);
    assert!(matches!(reached.1.as_slice(), [Reach::Found(p)] if p.contains("a*") && p.ends_with(".git/f")), "a-star walk: {:?}", reached.1);
    assert!(!allowed, "{why}");
    assert!(why.contains(FROZEN_REASON), "{why}");
}

#[test]
fn a_glob_named_directory_beside_a_symlink_is_still_listed() {
    let (_dir, root) = project();
    let star = format!("{root}/*");
    std::fs::create_dir(&star).expect("star directory");
    std::fs::create_dir(format!("{star}/.git")).expect("git");
    symlink("/etc/hosts", format!("{root}/out")).expect("sibling link");
    let root_for_walk = root.clone();
    let (reached, allowed, why) = on_hook_stack(move || {
        let _guard = pathctx::enter(ctx(&root_for_walk));
        let reached = reach_below("*", Walk::plain(true));
        let (allowed, why) = verdict_and_reason(&root_for_walk, "chmod -R 755 *");
        (reached, allowed, why)
    });
    assert!(matches!(reached.as_slice(), [Reach::Found(p)] if p.ends_with(".git/f")), "{reached:?}");
    assert!(!allowed, "{why}");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// A tree of glob-like names stays finite. An approval means the walk finished and found no
    /// frozen name; a planted one, or a walk that stopped early, is refused.
    #[test]
    fn a_glob_named_tree_terminates_and_approves_only_a_finished_walk(
        chain in prop::collection::vec(prop::sample::select(GLOB_NAMES), 1..5),
        plant in any::<bool>(),
    ) {
        let (_dir, root) = project();
        let mut at = root.clone();
        for name in &chain {
            at.push('/');
            at.push_str(name);
            std::fs::create_dir(&at).expect("directory");
        }
        if plant {
            std::fs::create_dir(format!("{at}/.git")).expect("git");
        }
        let _guard = pathctx::enter(ctx(&root));
        let literal = reach_below(".", Walk::plain(true));
        let globbed = reach_below("*", Walk::plain(true));
        let dot = crate::command_verdict("chmod -R 755 .").is_allowed();
        let star = crate::command_verdict("chmod -R 755 *").is_allowed();
        prop_assert!(finished(&literal) || !dot, "dot walk unfinished and approved: {literal:?} chain {chain:?}");
        prop_assert!(finished(&globbed) || !star, "glob walk unfinished and approved: {globbed:?} chain {chain:?}");
        if plant {
            prop_assert!(!dot, "dot approved with .git below {chain:?}: {literal:?}");
            prop_assert!(!star, "glob approved with .git below {chain:?}: {globbed:?}");
            prop_assert!(
                literal.iter().any(|r| matches!(r, Reach::Found(p) if p.contains(".git"))),
                "literal walk missed .git under {chain:?}: {literal:?}"
            );
            prop_assert!(
                globbed.iter().any(|r| matches!(r, Reach::Found(p) if p.contains(".git"))),
                "glob walk missed .git under {chain:?}: {globbed:?}"
            );
        } else if finished(&literal) && finished(&globbed) {
            prop_assert!(literal.is_empty(), "nothing frozen, literal walk returned {literal:?}");
            prop_assert!(globbed.is_empty(), "nothing frozen, glob walk returned {globbed:?}");
            prop_assert!(dot, "finished dot walk was refused for {chain:?}");
            prop_assert!(star, "finished glob walk was refused for {chain:?}");
        }
    }
}
