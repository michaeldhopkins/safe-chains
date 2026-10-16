use super::{Reach, frozen_below};
use crate::pathctx::{self, PathCtx};

fn project(dirs: &[&str]) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().expect("scratch project");
    let root = dir.path().canonicalize().expect("canonical").to_str().expect("utf-8").to_string();
    for d in dirs {
        std::fs::create_dir_all(format!("{root}/{d}")).expect("folder");
    }
    (dir, root)
}

fn within<T>(root: &str, f: impl FnOnce() -> T) -> T {
    let _ctx = pathctx::enter(PathCtx { cwd: Some(root.to_string()), root: Some(root.to_string()), session_id: None });
    f()
}

#[test]
fn the_root_reaches_its_own_metadata() {
    let (_d, root) = project(&[".git/hooks", "src"]);
    assert_eq!(within(&root, || frozen_below(".", true)), Some(Reach::Found("./.git/f".to_string())));
    assert_eq!(within(&root, || frozen_below("src", true)), None);
}

#[test]
fn a_nested_repository_is_found_below_a_plain_folder() {
    let (_d, root) = project(&["vendor/lib/.jj/repo", "src"]);
    assert_eq!(within(&root, || frozen_below("vendor", true)), Some(Reach::Found("vendor/lib/.jj/f".to_string())));
}

#[test]
fn a_hidden_name_is_skipped_when_the_walk_skips_hidden_entries() {
    let (_d, root) = project(&[".git/hooks", "src/.hg"]);
    assert_eq!(within(&root, || frozen_below(".", false)), None);
}

#[test]
fn an_envrc_file_counts() {
    let (_d, root) = project(&["sub"]);
    std::fs::write(format!("{root}/sub/.envrc"), "").expect("file");
    assert_eq!(within(&root, || frozen_below("sub", true)), Some(Reach::Found("sub/.envrc/f".to_string())));
}

#[test]
fn a_glob_reaches_what_its_matches_reach() {
    let (_d, root) = project(&["a/.git", "b"]);
    assert_eq!(within(&root, || frozen_below("*", true)), Some(Reach::Found("a/.git/f".to_string())));
    assert_eq!(within(&root, || frozen_below("b*", true)), None);
    assert_eq!(within(&root, || frozen_below("*/x", true)), Some(Reach::Unlisted("*/x/.git/f".to_string())));
}

#[test]
fn a_tree_too_large_to_list_is_judged_as_reaching() {
    let (_d, root) = project(&["big"]);
    for i in 0..crate::engine::resolve::frozen_walk::MAX_ENTRIES + 1 {
        std::fs::write(format!("{root}/big/{i}"), "").expect("file");
    }
    assert_eq!(within(&root, || frozen_below("big", true)), Some(Reach::Unlisted("big/.git/f".to_string())));
}

#[test]
fn a_linked_folder_is_not_walked_into() {
    let (_d, root) = project(&["src", "other/.git"]);
    std::os::unix::fs::symlink(format!("{root}/other"), format!("{root}/src/link")).expect("link");
    assert_eq!(within(&root, || frozen_below("src", true)), None);
}

#[test]
fn no_context_places_nothing() {
    assert_eq!(frozen_below(".", true), None);
}
