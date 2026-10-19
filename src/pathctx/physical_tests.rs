use super::{SYSTEM_LINKS, resolve};
use std::os::unix::fs::symlink;

fn tree() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let base = super::system_spelling(dir.path().canonicalize().unwrap().to_str().unwrap().to_string());
    std::fs::create_dir_all(format!("{base}/p/src")).unwrap();
    std::fs::create_dir_all(format!("{base}/o/deep")).unwrap();
    std::fs::write(format!("{base}/o/f"), "").unwrap();
    std::fs::write(format!("{base}/p/src/a"), "").unwrap();
    (dir, base)
}

#[test]
fn a_path_without_links_is_left_to_the_lexical_rules() {
    let (_dir, base) = tree();
    assert_eq!(resolve(&format!("{base}/p/src/a"), true), None);
    assert_eq!(resolve(&format!("{base}/p/src/missing/x"), true), None);
    assert_eq!(resolve("relative/path", true), None);
}

#[test]
fn a_link_is_replaced_by_its_target_and_dotdot_climbs_from_the_target() {
    let (_dir, base) = tree();
    symlink(format!("{base}/o/f"), format!("{base}/p/abs")).unwrap();
    symlink("../o/deep", format!("{base}/p/rel")).unwrap();
    assert_eq!(resolve(&format!("{base}/p/abs"), true), Some(format!("{base}/o/f")));
    assert_eq!(resolve(&format!("{base}/p/rel/new"), true), Some(format!("{base}/o/deep/new")));
    assert_eq!(resolve(&format!("{base}/p/rel/../f"), true), Some(format!("{base}/o/f")));
}

#[test]
fn a_rebind_keeps_the_last_link_unless_the_path_ends_in_a_slash() {
    let (_dir, base) = tree();
    symlink(format!("{base}/o/deep"), format!("{base}/p/d")).unwrap();
    assert_eq!(resolve(&format!("{base}/p/d"), false), None);
    assert_eq!(resolve(&format!("{base}/p/d/"), false), Some(format!("{base}/o/deep")));
    assert_eq!(resolve(&format!("{base}/p/d/x"), false), Some(format!("{base}/o/deep/x")));
}

#[test]
fn a_dangling_link_still_leads_to_its_target() {
    let (_dir, base) = tree();
    symlink(format!("{base}/o/not-yet"), format!("{base}/p/w")).unwrap();
    assert_eq!(resolve(&format!("{base}/p/w"), true), Some(format!("{base}/o/not-yet")));
}

#[test]
fn a_loop_is_unresolvable() {
    let (_dir, base) = tree();
    symlink(format!("{base}/p/b"), format!("{base}/p/a2")).unwrap();
    symlink(format!("{base}/p/a2"), format!("{base}/p/b")).unwrap();
    assert_eq!(resolve(&format!("{base}/p/a2"), true).as_deref(), Some(super::super::UNRESOLVED_CWD));
}

#[test]
fn a_symlink_whose_name_looks_like_a_glob_is_followed() {
    let (_dir, base) = tree();
    symlink(format!("{base}/o/f"), format!("{base}/p/foo[bar]")).unwrap();
    assert_eq!(resolve(&format!("{base}/p/foo[bar]"), true), Some(format!("{base}/o/f")));
    assert_eq!(resolve(&format!("{base}/p/foo[bar]/"), false), Some(format!("{base}/o/f")));
    assert_eq!(resolve(&format!("{base}/p/foo[bar]"), false), None);
}

#[test]
fn a_glob_through_a_link_reaches_the_link_target() {
    let (_dir, base) = tree();
    symlink(format!("{base}/o/f"), format!("{base}/p/rc")).unwrap();
    assert_eq!(resolve(&format!("{base}/p/r[c]"), true), Some(format!("{base}/o/f")));
    assert_eq!(resolve(&format!("{base}/p/s*/a"), true), None);
    assert_eq!(resolve(&format!("{base}/p/*"), true).as_deref(), Some(super::super::UNRESOLVED_CWD));
}

#[test]
fn a_link_target_is_taken_literally_not_as_a_glob() {
    let (_dir, base) = tree();
    symlink(format!("{base}/o/*"), format!("{base}/p/star")).unwrap();
    assert_eq!(resolve(&format!("{base}/p/star"), true), Some(format!("{base}/o/*")));
}

#[test]
fn the_macos_system_links_are_written_back_short() {
    for long in SYSTEM_LINKS {
        assert_eq!(super::system_spelling(format!("{long}/x")), format!("{}/x", &long["/private".len()..]));
    }
    assert_eq!(super::system_spelling("/private/etcetera".into()), "/private/etcetera");
    assert_eq!(super::system_spelling("/System/Volumes/Data/home/u".into()), "/home/u");
    assert_eq!(super::system_spelling("/System/Volumes/Data/private/tmp/x".into()), "/tmp/x");
    assert_eq!(super::system_spelling("/System/Volumes/Dataless".into()), "/System/Volumes/Dataless");
}

#[test]
fn a_glob_budget_counts_entries_across_nested_directories() {
    let (_dir, base) = tree();
    for d in 0..5 {
        let sub = format!("{base}/p/d{d}");
        std::fs::create_dir_all(&sub).unwrap();
        for f in 0..900 {
            std::fs::write(format!("{sub}/file-{f}-long-enough-to-expose-backtracking"), "").unwrap();
        }
    }
    assert_eq!(resolve(&format!("{base}/p/d*/******."), true).as_deref(), Some(super::super::UNRESOLVED_CWD));
    let tokens: Vec<_> = ["cargo", "--target-dir", "--output-dir=/**/******./.."]
        .into_iter()
        .map(|s| crate::parse::Token::from_raw(s.to_string()))
        .collect();
    assert!(crate::pathgate::should_deny("cargo", &tokens));
}
