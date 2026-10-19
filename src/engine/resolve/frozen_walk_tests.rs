use std::os::unix::fs::{PermissionsExt, symlink};

use super::{Reach, Walk, reach_below};
use crate::pathctx::{self, PathCtx};
use crate::{command_verdict_in, facet_breakdown};
use proptest::prelude::*;

/// The phrase already used when a walk finds a frozen name, including when it stops at its entry bound
/// and stands that name in for the tree it could not finish.
const FROZEN_REASON: &str = "changes your project's .git, hooks or CI settings";

const OPERAND_NAMES: &[&str] = &["[q]", "foo[bar]", "[*]", "a*"];
const FROZEN_NAMES: &[&str] = &[".git", ".jj", ".hg", ".envrc"];

/// A name the pattern matches that is not the pattern's own text. `foo[bar]` matches `foob`, not a
/// directory literally named `foo[bar]`.
fn glob_sibling(name: &str) -> &'static str {
    match name {
        "[q]" => "q",
        "foo[bar]" => "foob",
        "[*]" => "*",
        _ => "ab",
    }
}

struct Unseal(Vec<String>);

impl Drop for Unseal {
    fn drop(&mut self) {
        for path in &self.0 {
            let Ok(meta) = std::fs::metadata(path) else { continue };
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(path, perms).ok();
        }
    }
}

fn seal(path: &str) {
    let mut perms = std::fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o0);
    std::fs::set_permissions(path, perms).expect("seal");
}

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

#[test]
fn a_literal_directory_whose_name_looks_like_a_glob_is_opened() {
    let (_dir, root) = project();
    for name in ["foo[bar]", "[q]", "[*]", "sub/a[b]"] {
        let dir = format!("{root}/{name}");
        std::fs::create_dir_all(&dir).expect("literal directory");
        std::fs::create_dir(format!("{dir}/.git")).expect("git");
    }
    std::fs::create_dir_all(format!("{root}/plain[ok]/keep")).expect("clean literal");
    let root_for = root.clone();
    let (reached, verdicts, clean_find) = on_hook_stack(move || {
        let _guard = pathctx::enter(ctx(&root_for));
        let reached: Vec<_> = ["foo[bar]", "[q]", "[*]", "sub/a[b]"]
            .into_iter()
            .map(|name| (name, reach_below(name, Walk::plain(true))))
            .collect();
        let lines = [
            "chmod -R 755 'foo[bar]'", "cp -R 'foo[bar]' dest", "rm -rf 'foo[bar]'", "find 'foo[bar]' -delete", "chmod -R 755 '[q]'",
            "chmod -R 755 '[*]'", "chmod -R 755 'sub/a[b]'",
        ];
        let verdicts: Vec<_> = lines.iter().map(|line| (line.to_string(), verdict_and_reason(&root_for, line))).collect();
        let clean_find = verdict_and_reason(&root_for, "find 'plain[ok]' -delete");
        (reached, verdicts, clean_find)
    });
    for (name, found) in &reached {
        assert!(found.iter().any(|r| matches!(r, Reach::Found(p) if p.contains(".git"))), "literal {name} was not opened: {found:?}");
    }
    for (line, (allowed, why)) in &verdicts {
        assert!(!allowed, "{line} was approved over .git: {why}");
        // `find -delete` refuses through the inner `rm -r` and does not repeat that profile here.
        if !line.starts_with("find ") {
            assert!(why.contains(".git"), "{line}: {why}");
        }
    }
    assert!(clean_find.0, "find -delete of a literal directory with no frozen name was refused: {}", clean_find.1);
}

#[test]
fn a_literal_glob_directory_beside_a_matching_symlink_is_still_opened() {
    let (_dir, root) = project();
    std::fs::create_dir_all(format!("{root}/foo[bar]/.git")).expect("literal git");
    symlink("/etc/hosts", format!("{root}/foob")).expect("sibling that matches the pattern");
    let root_for = root.clone();
    let (reached, allowed, why) = on_hook_stack(move || {
        let _guard = pathctx::enter(ctx(&root_for));
        let reached = reach_below("foo[bar]", Walk::plain(true));
        let (allowed, why) = verdict_and_reason(&root_for, "chmod -R 755 'foo[bar]'");
        (reached, allowed, why)
    });
    assert!(
        matches!(reached.as_slice(), [Reach::Found(p)] if p.contains("foo[bar]") && p.ends_with(".git/f")),
        "sibling symlink hid the literal directory: {reached:?}"
    );
    assert!(!allowed, "{why}");
    assert!(why.contains(".git"), "{why}");
}

/// A project root that is itself a symlink, an unreadable directory named `foo[bar]`, and a sibling
/// the pattern matches. The literal directory is still refused.
#[test]
fn an_unreadable_literal_glob_directory_behind_a_root_symlink_is_refused() {
    let parent = tempfile::tempdir().expect("scratch");
    let real = parent.path().join("real");
    std::fs::create_dir(&real).expect("real");
    let alias = parent.path().join("alias");
    symlink(&real, &alias).expect("alias");
    let literal = alias.join("foo[bar]");
    std::fs::create_dir_all(literal.join(".git")).expect("git");
    let literal_path = literal.to_str().expect("utf-8").to_string();
    seal(&literal_path);
    symlink("/etc/hosts", alias.join("foob")).expect("sibling");
    let root = alias.to_str().expect("utf-8").to_string();
    let _unseal = Unseal(vec![literal_path]);
    let (reached, allowed, why) = on_hook_stack(move || {
        let _guard = pathctx::enter(ctx(&root));
        let reached = reach_below("foo[bar]", Walk::plain(true));
        let (allowed, why) = verdict_and_reason(&root, "chmod -R 755 'foo[bar]'");
        (reached, allowed, why)
    });
    assert!(reached.iter().any(|r| matches!(r, Reach::Unlisted(_))), "root symlink hid the unreadable literal directory: {reached:?}");
    assert!(!allowed, "{why}");
    assert!(why.contains(".git"), "{why}");
}

#[test]
fn a_glob_operand_walks_the_literal_directory_and_every_match() {
    let (_dir, root) = project();
    std::fs::create_dir_all(format!("{root}/foo[bar]/.git")).expect("literal git");
    std::fs::create_dir_all(format!("{root}/foob")).expect("pattern match");
    let root_for = root.clone();
    let (only_literal, both) = on_hook_stack(move || {
        let _guard = pathctx::enter(ctx(&root_for));
        let only_literal = reach_below("foo[bar]", Walk::plain(true));
        std::fs::create_dir(format!("{root_for}/foob/.git")).expect("match git");
        let both = reach_below("foo[bar]", Walk::plain(true));
        (only_literal, both)
    });
    assert!(
        matches!(only_literal.as_slice(), [Reach::Found(p)] if p.contains("foo[bar]") && p.ends_with(".git/f")),
        "literal directory missed: {only_literal:?}"
    );
    assert!(both.iter().any(|r| matches!(r, Reach::Found(p) if p.contains(".git"))), "union missed .git: {both:?}");

    let (_dir, root) = project();
    std::fs::create_dir_all(format!("{root}/foob/.git")).expect("match git");
    std::fs::create_dir_all(format!("{root}/foo[bar]/plain")).expect("clean literal");
    let root_for = root.clone();
    let only_match = on_hook_stack(move || {
        let _guard = pathctx::enter(ctx(&root_for));
        reach_below("foo[bar]", Walk::plain(true))
    });
    assert!(
        matches!(only_match.as_slice(), [Reach::Found(p)] if p.contains("foob") && p.ends_with(".git/f")),
        "pattern match missed: {only_match:?}"
    );
}

#[test]
fn an_unreadable_directory_is_unlisted() {
    let (_dir, root) = project();
    let secret = format!("{root}/secret");
    let nested = format!("{root}/parent/sealed");
    let globbed = format!("{root}/globbed");
    let bare = format!("{root}/bare");
    std::fs::create_dir_all(format!("{secret}/.git")).expect("git");
    std::fs::create_dir_all(format!("{nested}/.git")).expect("nested git");
    std::fs::create_dir_all(format!("{globbed}/child")).expect("glob child");
    std::fs::create_dir(&bare).expect("bare");
    for path in [&secret, &nested, &globbed, &bare] {
        seal(path);
    }
    let _unseal = Unseal(vec![secret, nested, globbed, bare]);
    let root_for = root.clone();
    let (top, child, glob, empty, chmod_top, chmod_parent, chmod_glob, chmod_bare) = on_hook_stack(move || {
        let _guard = pathctx::enter(ctx(&root_for));
        (
            reach_below("secret", Walk::plain(true)),
            reach_below("parent", Walk::plain(true)),
            reach_below("globbed/*", Walk::plain(true)),
            reach_below("bare", Walk::plain(true)),
            verdict_and_reason(&root_for, "chmod -R 755 secret"),
            verdict_and_reason(&root_for, "chmod -R 755 parent"),
            verdict_and_reason(&root_for, "chmod -R 755 'globbed/*'"),
            verdict_and_reason(&root_for, "chmod -R 755 bare"),
        )
    });
    for (label, reached) in [("secret", &top), ("parent", &child), ("globbed/*", &glob), ("bare", &empty)] {
        assert!(reached.iter().any(|r| matches!(r, Reach::Unlisted(_))), "{label} was treated as empty: {reached:?}");
    }
    for (label, (allowed, why)) in [("secret", &chmod_top), ("parent", &chmod_parent), ("globbed/*", &chmod_glob), ("bare", &chmod_bare)] {
        assert!(!allowed, "chmod -R of unreadable {label} was approved: {why}");
        assert!(why.contains(".git"), "{label}: {why}");
    }
}

#[test]
fn a_missing_path_and_a_file_are_not_unreadable_directories() {
    let (_dir, root) = project();
    std::fs::write(format!("{root}/file[name]"), "x").expect("file");
    let root_for = root.clone();
    let (missing, file, chmod_missing, chmod_file) = on_hook_stack(move || {
        let _guard = pathctx::enter(ctx(&root_for));
        (
            reach_below("missing", Walk::plain(true)),
            reach_below("file[name]", Walk::plain(true)),
            verdict_and_reason(&root_for, "chmod -R 755 missing"),
            verdict_and_reason(&root_for, "chmod -R 755 'file[name]'"),
        )
    });
    assert!(missing.is_empty(), "{missing:?}");
    assert!(file.is_empty(), "{file:?}");
    assert!(chmod_missing.0, "{}", chmod_missing.1);
    assert!(chmod_file.0, "{}", chmod_file.1);
}

#[test]
fn a_fifty_level_chain_of_star_directories_is_walked_through_star() {
    let (_dir, root) = project();
    let mut at = root.clone();
    for _ in 0..50 {
        at.push_str("/*");
        std::fs::create_dir(&at).expect("star directory");
    }
    std::fs::create_dir(format!("{at}/.git")).expect("git");
    let root_for = root.clone();
    let (allowed, reached) = on_hook_stack(move || {
        let _guard = pathctx::enter(ctx(&root_for));
        let reached = reach_below("*", Walk::plain(true));
        let allowed = crate::command_verdict("chmod -R 755 '*'").is_allowed();
        (allowed, reached)
    });
    assert!(!allowed, "fifty stars hid .git and were auto-approved: {reached:?}");
    assert!(
        reached.iter().any(|r| matches!(r, Reach::Found(p) if p.contains(".git"))),
        "walk through * did not reach the bottom: {reached:?}"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(80))]

    /// An operand drawn from the glob-like names is the literal directory when that exists, plus
    /// every match. A frozen name anywhere under either is refused. An unreadable directory is
    /// unfinished, so it is refused too.
    #[test]
    fn a_literal_glob_operand_and_an_unreadable_directory_are_not_approved(
        name in prop::sample::select(OPERAND_NAMES),
        depth in 0usize..3,
        plant_at in 0usize..4,
        frozen in prop::sample::select(FROZEN_NAMES),
        sibling_frozen in any::<bool>(),
        seal_tree in any::<bool>(),
    ) {
        let (_dir, root) = project();
        let mut at = format!("{root}/{name}");
        std::fs::create_dir_all(&at).expect("operand directory");
        let mut planted = plant_at == 0;
        if planted {
            std::fs::create_dir(format!("{at}/{frozen}")).expect("frozen");
        }
        for i in 1..=depth {
            at = format!("{at}/n");
            std::fs::create_dir(&at).expect("nested");
            if i == plant_at {
                std::fs::create_dir(format!("{at}/{frozen}")).expect("frozen");
                planted = true;
            }
        }
        let sibling = glob_sibling(name);
        let sibling_dir = format!("{root}/{sibling}");
        std::fs::create_dir_all(&sibling_dir).expect("glob match");
        if sibling_frozen {
            std::fs::create_dir(format!("{sibling_dir}/.git")).expect("match git");
        }
        let sealed = seal_tree.then(|| {
            let path = format!("{root}/sealed");
            std::fs::create_dir_all(format!("{path}/.git")).expect("sealed git");
            seal(&path);
            path
        });
        let _unseal = Unseal(sealed.clone().into_iter().collect());
        let _guard = pathctx::enter(ctx(&root));
        let reached = reach_below(name, Walk::plain(true));
        let chmod = crate::command_verdict(&format!("chmod -R 755 '{name}'")).is_allowed();
        prop_assert!(finished(&reached) || !chmod, "unfinished walk of {name} was approved: {reached:?}");
        if planted || sibling_frozen {
            prop_assert!(!chmod, "approved {name} with a frozen name below it: {reached:?}");
            prop_assert!(
                reached.iter().any(|r| matches!(r, Reach::Found(_))),
                "walk of {name} missed a frozen name (planted {planted} at {plant_at}, sibling {sibling_frozen}): {reached:?}"
            );
        } else if finished(&reached) {
            prop_assert!(reached.is_empty(), "nothing frozen under {name}, walk returned {reached:?}");
            prop_assert!(chmod, "finished walk of clean {name} was refused: {reached:?}");
        }
        if sealed.is_some() {
            let unreadable = reach_below("sealed", Walk::plain(true));
            let allowed = crate::command_verdict("chmod -R 755 sealed").is_allowed();
            prop_assert!(
                unreadable.iter().any(|r| matches!(r, Reach::Unlisted(_))),
                "unreadable directory was treated as empty: {unreadable:?}"
            );
            prop_assert!(!allowed, "chmod of an unreadable directory was approved: {unreadable:?}");
        }
    }
}
