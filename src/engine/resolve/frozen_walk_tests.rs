use std::os::unix::ffi::OsStringExt;
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

/// The spelling a resolved path comes back with. macOS rewrites `/private/{tmp,var,etc}` to the
/// short form, and a project root spelled the other way classifies that path as outside.
fn spelled_root(canonical: &str) -> String {
    match canonical.strip_prefix("/private") {
        Some(rest) if rest.starts_with("/var/") || rest.starts_with("/tmp/") || rest.starts_with("/etc/") => rest.to_string(),
        _ => canonical.to_string(),
    }
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

/// `rm -rf a*` where `a*` is mode 000, `a0/.git` is readable, and the parent has more matches than
/// the expander keeps. The shell still deletes `a0`. The same shape with `foo[bar]` over the entry cap.
#[test]
fn an_unquoted_glob_does_not_delete_a_readable_match_the_literal_never_opened() {
    let (_dir, root) = project();
    let literal = format!("{root}/a*");
    std::fs::create_dir(&literal).expect("literal");
    seal(&literal);
    std::fs::create_dir_all(format!("{root}/a0/.git")).expect("match git");
    for i in 1..260 {
        std::fs::write(format!("{root}/a{i}"), "").expect("filler");
    }
    let _unseal = Unseal(vec![literal]);
    let root_for = root.clone();
    let (rm_glob, rm_match, chmod_glob, reached) = on_hook_stack(move || {
        let _guard = pathctx::enter(ctx(&root_for));
        let reached = reach_below("a*", Walk::plain(true));
        (
            verdict_and_reason(&root_for, "rm -rf a*"),
            verdict_and_reason(&root_for, "rm -rf a0"),
            verdict_and_reason(&root_for, "chmod -R 755 a*"),
            reached,
        )
    });
    assert!(!rm_match.0, "rm of the readable match was approved: {}", rm_match.1);
    assert!(!chmod_glob.0, "chmod of the glob was approved: {}", chmod_glob.1);
    assert!(
        reached.iter().any(|r| matches!(r, Reach::Found(p) if p.contains("a0") && p.ends_with(".git/f"))),
        "literal unlisted hid a0/.git: {reached:?}"
    );
    assert!(!rm_glob.0, "rm of the unexpanded glob deleted a readable match: {}", rm_glob.1);

    let (_dir, root) = project();
    let literal = format!("{root}/foo[bar]");
    std::fs::create_dir(&literal).expect("literal");
    seal(&literal);
    std::fs::create_dir_all(format!("{root}/foob/.git")).expect("match git");
    for i in 0..4095 {
        std::fs::write(format!("{root}/p{i}"), "").expect("filler");
    }
    let _unseal = Unseal(vec![literal]);
    let root_for = root.clone();
    let (rm_glob, rm_match) =
        on_hook_stack(move || (verdict_and_reason(&root_for, "rm -rf foo[bar]"), verdict_and_reason(&root_for, "rm -rf foob")));
    assert!(!rm_match.0, "rm of foob was approved: {}", rm_match.1);
    assert!(!rm_glob.0, "rm of unquoted foo[bar] deleted foob/.git: {}", rm_glob.1);
}

/// An absolute glob is not expanded, and neither is one whose listing gave up. The operand is a
/// symlink onto the only match, which holds `.git`. The shell deletes or chmods that match.
#[test]
fn an_unexpanded_glob_does_not_approve_the_match_behind_a_symlink() {
    let (_dir, root) = project();
    std::fs::create_dir_all(format!("{root}/foob/.git")).expect("git");
    symlink("foob", format!("{root}/foo[bar]")).expect("operand symlink");
    let root_for = root.clone();
    let (rm_abs, chmod_abs) = on_hook_stack(move || {
        (
            verdict_and_reason(&root_for, &format!("rm -rf {root_for}/foo[bar]")),
            verdict_and_reason(&root_for, &format!("chmod -R 755 {root_for}/foo[bar]")),
        )
    });
    assert!(!rm_abs.0, "absolute glob rm was approved: {}", rm_abs.1);
    assert!(!chmod_abs.0, "absolute glob chmod was approved: {}", chmod_abs.1);

    let (_dir, root) = project();
    std::fs::create_dir_all(format!("{root}/foob/.git")).expect("git");
    symlink("foob", format!("{root}/foo[bar]")).expect("operand symlink");
    for i in 0..4096 {
        std::fs::write(format!("{root}/p{i}"), "").expect("filler");
    }
    let root_for = root.clone();
    let (rm_entries, chmod_entries) =
        on_hook_stack(move || (verdict_and_reason(&root_for, "rm -rf foo[bar]"), verdict_and_reason(&root_for, "chmod -R 755 foo[bar]")));
    assert!(!rm_entries.0, "rm was approved after the entry cap: {}", rm_entries.1);
    assert!(!chmod_entries.0, "chmod was approved after the entry cap: {}", chmod_entries.1);

    let (_dir, root) = project();
    std::fs::create_dir_all(format!("{root}/a0/.git")).expect("git");
    symlink("a0", format!("{root}/a*")).expect("operand symlink");
    for i in 1..257 {
        std::fs::write(format!("{root}/a{i}"), "").expect("match");
    }
    let root_for = root.clone();
    let (rm_matches, chmod_matches) =
        on_hook_stack(move || (verdict_and_reason(&root_for, "rm -rf a*"), verdict_and_reason(&root_for, "chmod -R 755 a*")));
    assert!(!rm_matches.0, "rm was approved after the match cap: {}", rm_matches.1);
    assert!(!chmod_matches.0, "chmod was approved after the match cap: {}", chmod_matches.1);

    let (_dir, root) = project();
    std::fs::create_dir_all(format!("{root}/foob/.git")).expect("git");
    symlink("foob", format!("{root}/foo[bar]")).expect("operand symlink");
    let bad = std::ffi::OsString::from_vec({
        let mut bytes = b"zz-".to_vec();
        bytes.push(0xff);
        bytes
    });
    let created = std::fs::File::create(std::path::Path::new(&root).join(&bad));
    if created.is_err() {
        // This filesystem rejects a non-UTF-8 file name, so a listing cannot give up on one here.
        return;
    }
    let root_for = root.clone();
    let (rm_bytes, chmod_bytes) =
        on_hook_stack(move || (verdict_and_reason(&root_for, "rm -rf foo[bar]"), verdict_and_reason(&root_for, "chmod -R 755 foo[bar]")));
    assert!(!rm_bytes.0, "rm was approved beside a non-UTF-8 name: {}", rm_bytes.1);
    assert!(!chmod_bytes.0, "chmod was approved beside a non-UTF-8 name: {}", chmod_bytes.1);
}

/// `foo[bar]` holds more entries than the walk keeps, the parent has more names than expansion
/// keeps, and `foob/.git` is the match. `rm -rf foo[bar]` deletes `foob`.
#[test]
fn a_literal_directory_budget_does_not_let_rm_delete_a_sibling_match() {
    let (_dir, root) = project();
    let literal = format!("{root}/foo[bar]");
    std::fs::create_dir(&literal).expect("literal");
    for i in 0..=super::MAX_ENTRIES {
        std::fs::write(format!("{literal}/{i}"), "").expect("literal file");
    }
    std::fs::create_dir_all(format!("{root}/foob/.git")).expect("match git");
    for i in 0..4100 {
        std::fs::write(format!("{root}/p{i}"), "").expect("sibling");
    }
    let root_for = root.clone();
    let (rm_glob, chmod_glob) =
        on_hook_stack(move || (verdict_and_reason(&root_for, "rm -rf foo[bar]"), verdict_and_reason(&root_for, "chmod -R 755 foo[bar]")));
    assert!(!chmod_glob.0, "chmod was approved when the literal spent the budget: {}", chmod_glob.1);
    assert!(!rm_glob.0, "rm deleted the sibling match the budget hid: {}", rm_glob.1);
}

/// `foo[bar]` points at `.git` and `foob` is a symlink to an ordinary file. The quoted operand is
/// the first symlink. `chmod` and `rm -rf` of it change `.git`.
#[test]
fn a_quoted_symlink_is_judged_when_another_match_is_a_symlink() {
    let (_dir, canonical) = project();
    let root = spelled_root(&canonical);
    let git = format!("{root}/repo/.git");
    std::fs::create_dir_all(format!("{git}/hooks")).expect("git");
    let hook = format!("{git}/hooks/pre-commit");
    std::fs::write(&hook, "#!/bin/sh\n").expect("hook");
    let mut perms = std::fs::metadata(&hook).expect("hook metadata").permissions();
    perms.set_mode(0o700);
    std::fs::set_permissions(&hook, perms).expect("hook mode");
    symlink("repo/.git", format!("{root}/foo[bar]")).expect("operand symlink");
    std::fs::write(format!("{root}/readme"), "ok").expect("readme");
    symlink("readme", format!("{root}/foob")).expect("harmless match");
    let root_for = root.clone();
    let verdicts =
        on_hook_stack(move || ["chmod 755 'foo[bar]'", "rm -rf 'foo[bar]/'"].map(|line| (line, verdict_and_reason(&root_for, line))));
    for (line, (allowed, why)) in &verdicts {
        assert!(!*allowed, "{line} was approved over a symlink to .git: {why}");
        assert!(why.contains(".git"), "{line} was refused without judging .git: {why}");
    }
}

/// A mode-000 child beside a readable `.git` must not hide it, whatever order `read_dir` uses.
/// A tree whose only `.git` is inside the sealed directory stays allowed: removal cannot list it
/// and does not plant a hook.
#[test]
fn an_unreadable_child_does_not_hide_a_readable_git() {
    let (_dir, root) = project();
    let proj = format!("{root}/proj");
    let sealed = format!("{proj}/sealed");
    std::fs::create_dir_all(format!("{proj}/repo/.git")).expect("git");
    std::fs::create_dir(&sealed).expect("sealed");
    seal(&sealed);
    let _unseal = Unseal(vec![sealed]);
    let root_for = root.clone();
    let (rm_proj, chmod_proj, find_proj, rm_only, reached) = on_hook_stack(move || {
        let only = format!("{root_for}/only");
        std::fs::create_dir_all(format!("{only}/.git")).expect("only git");
        seal(&only);
        let _guard = pathctx::enter(ctx(&root_for));
        let reached = reach_below("proj", Walk::plain(true));
        let verdicts = (
            verdict_and_reason(&root_for, "rm -rf proj"),
            verdict_and_reason(&root_for, "chmod -R 755 proj"),
            verdict_and_reason(&root_for, "find proj -delete"),
            verdict_and_reason(&root_for, "rm -rf only"),
            reached,
        );
        let mut perms = std::fs::metadata(&only).expect("metadata").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&only, perms).expect("unseal only");
        verdicts
    });
    assert!(reached.iter().any(|r| matches!(r, Reach::Found(p) if p.contains("repo/.git"))), "sealed sibling hid repo/.git: {reached:?}");
    assert!(!chmod_proj.0, "chmod of the mixed tree was approved: {}", chmod_proj.1);
    assert!(!find_proj.0, "find -delete of the mixed tree was approved: {}", find_proj.1);
    assert!(rm_only.0, "rm of a sealed tree whose only .git is inside it was refused: {}", rm_only.1);
    assert!(!rm_proj.0, "rm deleted a readable .git hidden by a sealed sibling: {}", rm_proj.1);
}

/// A symlink whose name contains `[` used to skip `lstat`, so `chmod` and `rm -rf` of it never saw
/// the `.git` it points at. `cp -RL` of that link stays allowed: the walk of `.git` finds no nested
/// frozen name.
#[test]
fn a_glob_named_symlink_to_git_is_judged_as_git() {
    let (_dir, root) = project();
    let git = format!("{root}/repo/.git");
    std::fs::create_dir_all(format!("{git}/hooks")).expect("git");
    std::fs::write(format!("{git}/hooks/pre-commit"), "#!/bin/sh\n").expect("hook");
    symlink(&git, format!("{root}/foo[bar]")).expect("glob link");
    symlink(&git, format!("{root}/link")).expect("plain link");
    let root_for = root.clone();
    let verdicts = on_hook_stack(move || {
        [
            "chmod -R 755 'foo[bar]/'", "rm -rf 'foo[bar]/'", "chmod 755 'foo[bar]'", "chmod -R 755 link/", "rm -rf link/",
            "chmod 755 link", "cp -RL 'foo[bar]' dest", "cp -RL link dest",
        ]
        .map(|line| (line, verdict_and_reason(&root_for, line)))
    });
    for (line, (allowed, why)) in &verdicts {
        if line.starts_with("cp ") {
            assert!(*allowed, "{line} was refused: {why}");
        } else {
            assert!(!*allowed, "{line} was approved over a symlink to .git: {why}");
        }
    }
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
    /// unfinished, so it is refused too. The operand may be a symlink, an absolute path, or sit
    /// beside a match that is itself a symlink.
    #[test]
    fn a_literal_glob_operand_and_an_unreadable_directory_are_not_approved(
        name in prop::sample::select(OPERAND_NAMES),
        depth in 0usize..3,
        plant_at in 0usize..4,
        frozen in prop::sample::select(FROZEN_NAMES),
        sibling_frozen in any::<bool>(),
        seal_tree in any::<bool>(),
        symlink_operand in any::<bool>(),
        absolute in any::<bool>(),
        match_is_symlink in any::<bool>(),
    ) {
        let (_dir, canonical) = project();
        let root = spelled_root(&canonical);
        let sibling = glob_sibling(name);
        let sibling_path = format!("{root}/{sibling}");
        let match_dir = if match_is_symlink {
            let payload = format!("{root}/payload");
            std::fs::create_dir_all(&payload).expect("payload");
            symlink("payload", &sibling_path).expect("match symlink");
            payload
        } else {
            std::fs::create_dir_all(&sibling_path).expect("glob match");
            sibling_path
        };
        if sibling_frozen {
            let git = format!("{match_dir}/.git");
            if !std::path::Path::new(&git).exists() {
                std::fs::create_dir(&git).expect("match git");
            }
        }
        let mut planted = false;
        if symlink_operand {
            symlink(sibling, format!("{root}/{name}")).expect("operand symlink");
            if plant_at == 0 || (1..=depth).contains(&plant_at) {
                let dest = format!("{match_dir}/{frozen}");
                if !std::path::Path::new(&dest).exists() {
                    std::fs::create_dir(&dest).expect("frozen");
                }
                planted = true;
            }
        } else {
            let mut at = format!("{root}/{name}");
            std::fs::create_dir_all(&at).expect("operand directory");
            planted = plant_at == 0;
            if planted {
                let dest = format!("{at}/{frozen}");
                if !std::path::Path::new(&dest).exists() {
                    std::fs::create_dir(&dest).expect("frozen");
                }
            }
            for i in 1..=depth {
                at = format!("{at}/n");
                std::fs::create_dir(&at).expect("nested");
                if i == plant_at {
                    std::fs::create_dir(format!("{at}/{frozen}")).expect("frozen");
                    planted = true;
                }
            }
        }
        let sealed = seal_tree.then(|| {
            let path = format!("{root}/sealed");
            std::fs::create_dir_all(format!("{path}/.git")).expect("sealed git");
            seal(&path);
            path
        });
        let _unseal = Unseal(sealed.clone().into_iter().collect());
        let _guard = pathctx::enter(ctx(&root));
        let operand = if absolute { format!("{root}/{name}") } else { name.to_string() };
        let reached = reach_below(&operand, Walk::plain(true));
        let chmod = crate::command_verdict(&format!("chmod -R 755 '{operand}'")).is_allowed();
        prop_assert!(finished(&reached) || !chmod, "unfinished walk of {operand} was approved: {reached:?}");
        if planted || sibling_frozen {
            prop_assert!(!chmod, "approved {operand} with a frozen name below it: {reached:?}");
            prop_assert!(
                reached.iter().any(|r| matches!(r, Reach::Found(_) | Reach::Unlisted(_))),
                "walk of {operand} missed a frozen name (planted {planted} at {plant_at}, sibling {sibling_frozen}, link {symlink_operand}, absolute {absolute}, match link {match_is_symlink}): {reached:?}"
            );
        } else if finished(&reached) {
            prop_assert!(reached.is_empty(), "nothing frozen under {operand}, walk returned {reached:?}");
            prop_assert!(!chmod, "clean {operand} beside another match was approved: {reached:?}");
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
