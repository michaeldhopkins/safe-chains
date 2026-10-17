use super::{Reach, frozen_below};
use crate::pathctx::{self, PathCtx};

fn project_in(dir: tempfile::TempDir, dirs: &[&str]) -> (tempfile::TempDir, String) {
    let base = dir.path().canonicalize().expect("canonical").to_str().expect("utf-8").to_string();
    let root = format!("{base}/work");
    std::fs::create_dir_all(&root).expect("work folder");
    for d in dirs {
        std::fs::create_dir_all(format!("{root}/{d}")).expect("folder");
    }
    (dir, root)
}

fn project(dirs: &[&str]) -> (tempfile::TempDir, String) {
    project_in(tempfile::tempdir().expect("scratch project"), dirs)
}

fn outside_project(dirs: &[&str]) -> (tempfile::TempDir, String) {
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let base = if std::fs::create_dir_all(&target)
        .and_then(|()| target.canonicalize())
        .is_ok_and(|p| p.to_str().is_some_and(|s| !crate::pathctx::under_temp_root(&format!("{s}/x"))))
    {
        target
    } else {
        let cache = std::path::PathBuf::from(std::env::var("HOME").expect("HOME")).join(".cache");
        std::fs::create_dir_all(&cache).expect("cache folder");
        cache
    };
    let dir = tempfile::Builder::new().prefix("sc-").tempdir_in(base).expect("outside project");
    project_in(dir, dirs)
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
    use crate::engine::resolve::frozen_walk::{MAX_ENTRIES, Walk, reach_below};
    let (_d, root) = project(&["big/d0", "big/d1", "big/d2", "big/d3", "big/d4"]);
    for d in 0..5 {
        for i in 0..MAX_ENTRIES / 4 + 1 {
            std::fs::write(format!("{root}/big/d{d}/{i}"), "").expect("file");
        }
    }
    assert_eq!(within(&root, || frozen_below("big", true)), Some(Reach::Unlisted("big/.git/f".to_string())));
    assert!(matches!(within(&root, || frozen_below("big/d*", true)), Some(Reach::Unlisted(_))));
    assert!(
        within(&root, || reach_below("big/d*", Walk::follow()))
            .iter()
            .any(|r| matches!(r, Reach::Unlisted(_)))
    );
}

#[test]
fn a_linked_folder_is_not_walked_into() {
    let (_d, root) = project(&["src", "other/.git"]);
    std::os::unix::fs::symlink(format!("{root}/other"), format!("{root}/src/link")).expect("link");
    assert_eq!(within(&root, || frozen_below("src", true)), None);
}

#[test]
fn an_ungranted_folder_outside_the_workspace_is_not_walked() {
    use crate::engine::resolve::regions::with_grants;
    let (_d, root) = project(&["src"]);
    let (_scratch_d, scratch) = project_in(tempfile::tempdir_in("/tmp").expect("tmp project"), &[".git/hooks"]);
    assert_eq!(within(&root, || frozen_below(&scratch, true)), Some(Reach::Found(format!("{scratch}/.git/f"))));
    let (_outside_d, outside) = outside_project(&[".git/hooks"]);
    assert_eq!(within(&root, || frozen_below(&outside, true)), None);
    assert_eq!(within(&root, || frozen_below(&format!("{outside}/*"), true)), None);
    let granted = crate::engine::resolve::locus::canonicalize(&outside);
    let expected = Some(Reach::Found(format!("{outside}/.git/f")));
    assert_eq!(within(&root, || with_grants(&[(granted.as_ref(), true, false)], || frozen_below(&outside, true))), expected);
    assert_eq!(within(&root, || with_grants(&[(granted.as_ref(), false, true)], || frozen_below(&outside, true))), expected);
}

#[test]
fn a_symlink_named_f_or_a_shielded_segment_inside_the_workspace_does_not_skip_the_walk() {
    let (_d, root) = project(&[".git/hooks", ".docker/.git/hooks"]);
    std::os::unix::fs::symlink("/etc/hosts", format!("{root}/f")).expect("link");
    assert_eq!(within(&root, || frozen_below(".", true)), Some(Reach::Found("./.git/f".to_string())));
    assert_eq!(within(&root, || frozen_below(".d*", true)), Some(Reach::Found(".docker/.git/f".to_string())));
}

#[test]
fn no_context_places_nothing() {
    assert_eq!(frozen_below(".", true), None);
}
