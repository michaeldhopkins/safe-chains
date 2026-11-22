use super::{PathCtx, anchor::Use, enter, note_reported_cwd, place::{Here, here, on_the_link, place}};
use std::os::unix::fs::symlink;

/// A project with `src/`, and a folder outside it holding `secret`, both under one canonical base.
fn tree() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let base = super::place::physical::system_spelling(dir.path().canonicalize().unwrap().to_str().unwrap().to_string());
    std::fs::create_dir_all(format!("{base}/proj/src")).unwrap();
    std::fs::create_dir_all(format!("{base}/out")).unwrap();
    std::fs::write(format!("{base}/proj/src/a.rs"), "").unwrap();
    std::fs::write(format!("{base}/out/secret"), "").unwrap();
    (dir, base)
}

fn inside(base: &str) -> super::Guard {
    let root = format!("{base}/proj");
    enter(PathCtx { cwd: Some(root.clone()), root: Some(root), session_id: None })
}

#[test]
fn a_path_naming_a_variable_is_unplaced_and_judged_as_written() {
    let (_dir, base) = tree();
    let _ctx = inside(&base);
    let placed = place("$HOME/x", None);
    assert_eq!((placed.judged(), placed.spelled()), ("$HOME/x", "$HOME/x"));
}

#[test]
fn an_empty_path_places_as_empty() {
    assert_eq!(place("", None).judged(), "");
}

#[test]
fn with_no_context_a_relative_path_stays_as_written() {
    let _ctx = enter(PathCtx::default());
    let placed = place("src/a.rs", Some(Use::Read));
    assert_eq!((placed.judged(), placed.spelled()), ("src/a.rs", "src/a.rs"));
}

#[test]
fn a_path_inside_the_project_is_judged_relative_to_it() {
    let (_dir, base) = tree();
    let _ctx = inside(&base);
    for path in ["src/a.rs", "./src/a.rs", &format!("{base}/proj/src/a.rs"), "src/../src/a.rs"] {
        assert_eq!(place(path, Some(Use::Read)).judged(), "src/a.rs", "{path}");
    }
}

#[test]
fn a_link_out_of_the_project_is_judged_where_it_leads_and_spelled_where_it_stands() {
    let (_dir, base) = tree();
    symlink(format!("{base}/out"), format!("{base}/proj/lnk")).unwrap();
    let _ctx = inside(&base);
    let placed = place("lnk/secret", Some(Use::Read));
    assert_eq!(placed.judged(), format!("{base}/out/secret"));
    assert_eq!(placed.spelled(), "lnk/secret");
}

#[test]
fn a_dotdot_after_a_link_climbs_from_where_the_link_leads() {
    let (_dir, base) = tree();
    symlink(format!("{base}/out"), format!("{base}/proj/lnk")).unwrap();
    let _ctx = inside(&base);
    let placed = place("lnk/..", Some(Use::Read));
    assert_eq!(placed.judged(), base);
    assert_eq!(placed.spelled(), ".");
}

#[test]
fn a_rebind_keeps_the_last_link_and_a_read_follows_it() {
    let (_dir, base) = tree();
    symlink(format!("{base}/out/secret"), format!("{base}/proj/lnk")).unwrap();
    let _ctx = inside(&base);
    assert_eq!(place("lnk", Some(Use::Rebind)).judged(), "lnk");
    assert_eq!(place("lnk", Some(Use::Read)).judged(), format!("{base}/out/secret"));
    assert_eq!(on_the_link(|| place("lnk", Some(Use::Read))).judged(), "lnk");
    assert_eq!(on_the_link(|| place("lnk/", Some(Use::Read))).judged(), format!("{base}/out/secret"));
}

#[test]
fn the_on_the_link_switch_is_restored_after_the_judge_returns() {
    let (_dir, base) = tree();
    symlink(format!("{base}/out/secret"), format!("{base}/proj/lnk")).unwrap();
    let _ctx = inside(&base);
    on_the_link(|| ());
    assert_eq!(place("lnk", Some(Use::Read)).judged(), format!("{base}/out/secret"));
}

#[test]
fn a_component_that_does_not_exist_is_kept_as_written() {
    let (_dir, base) = tree();
    let _ctx = inside(&base);
    assert_eq!(place("new/dir/file", Some(Use::Write)).judged(), "new/dir/file");
}

#[test]
fn a_home_path_inside_the_project_root_is_the_projects_own() {
    let (_dir, base) = tree();
    let _ctx = inside(&base);
    let placed = place(&format!("{base}/proj/src"), None);
    assert_eq!(placed.judged(), "src");
}

#[test]
fn the_reported_folder_is_placed_once_in_both_readings() {
    let (_dir, base) = tree();
    symlink(format!("{base}/out"), format!("{base}/proj/lnk")).unwrap();
    let _ctx = enter(PathCtx::default());
    note_reported_cwd(Some(format!("{base}/proj/lnk")));
    assert_eq!(here(), Some(Here { logical: format!("{base}/proj/lnk"), physical: format!("{base}/out") }));
    note_reported_cwd(Some("relative".to_string()));
    assert_eq!(here(), None, "a relative report names no folder");
    note_reported_cwd(Some("/work/$HOME".to_string()));
    assert_eq!(here(), None, "a report holding a variable names no folder");
}

#[test]
fn entering_a_context_starts_with_no_reported_folder_and_restores_the_last_one() {
    let (_dir, base) = tree();
    let _outer = enter(PathCtx::default());
    note_reported_cwd(Some(format!("{base}/proj")));
    let before = here();
    assert!(before.is_some());
    {
        let _inner = enter(PathCtx::default());
        assert_eq!(here(), None);
    }
    assert_eq!(here(), before);
}
