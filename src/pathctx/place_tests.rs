use super::{
    PathCtx,
    anchor::Use,
    enter, note_reported_cwd,
    place::{Here, here, on_the_link, place},
};
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
    note_reported_cwd(Some(format!("{base}/proj/lnk")), true);
    assert_eq!(here(), Some(Here { logical: format!("{base}/proj/lnk"), physical: format!("{base}/out"), runs_there: true }));
    note_reported_cwd(Some("relative".to_string()), true);
    assert_eq!(here(), None, "a relative report names no folder");
    note_reported_cwd(Some("/work/$HOME".to_string()), true);
    assert_eq!(here(), None, "a report holding a variable names no folder");
}

#[test]
fn entering_a_context_starts_with_no_reported_folder_and_restores_the_last_one() {
    let (_dir, base) = tree();
    let _outer = enter(PathCtx::default());
    note_reported_cwd(Some(format!("{base}/proj")), true);
    let before = here();
    assert!(before.is_some());
    {
        let _inner = enter(PathCtx::default());
        assert_eq!(here(), None);
    }
    assert_eq!(here(), before);
}

#[test]
fn a_reported_folder_the_command_does_not_run_in_is_never_the_folder_on_disk() {
    let (_dir, base) = tree();
    let unknown = format!("{}/sub", crate::targets::UNKNOWN_WORKDIR);
    for (runs_there, expected) in [(true, Some(format!("{base}/proj/sub"))), (false, None)] {
        let _ctx = enter(PathCtx::default());
        note_reported_cwd(Some(format!("{base}/proj")), runs_there);
        assert_eq!(super::place::folder_below(&unknown), expected, "runs_there {runs_there}");
        assert_eq!(
            super::join_cwd_chdir(Some(&unknown), ".."),
            expected
                .map(|_| format!("{base}/proj"))
                .or_else(|| super::join_cwd_mode(Some(&unknown), "..", true))
        );
        assert!(here().is_some(), "the folder is still placed, so a link in it can add a refusal");
    }
}

/// Links in a project to the folders that run code or hold agent settings, with a file under each.
const LINKED: [(&str, &str, &str); 4] = [
    ("glink", ".git", "hooks/pre-commit"),
    ("clink", ".codex", "hooks.json"),
    ("alink", ".claude", "settings.json"),
    ("ilink", ".agents", "skills/x.md"),
];

/// A stand-in home under the build folder, which has no temp-folder rule of its own (and exists on every platform).
fn linked_home() -> (tempfile::TempDir, String) {
    let parent = concat!(env!("CARGO_MANIFEST_DIR"), "/target");
    std::fs::create_dir_all(parent).unwrap();
    let dir = tempfile::Builder::new().prefix("sc-place-").tempdir_in(parent).unwrap();
    let home = super::place::physical::system_spelling(dir.path().canonicalize().unwrap().to_str().unwrap().to_string());
    std::fs::create_dir_all(format!("{home}/scfixture/app/src")).unwrap();
    for (link, target, _) in LINKED {
        std::fs::create_dir_all(format!("{home}/scfixture/app/{target}")).unwrap();
        symlink(target, format!("{home}/scfixture/app/{link}")).unwrap();
    }
    (dir, home)
}

fn spellings(home: &str, link: &str, rest: &str) -> Vec<String> {
    vec![
        format!("{link}/{rest}"),
        format!("./{link}/{rest}"),
        format!("{home}/scfixture/app/{link}/{rest}"),
        format!("~/scfixture/app/{link}/{rest}"),
        format!("~/scfixture/app/./{link}/{rest}"),
        format!("~/scfixture/app/src/{}/{link}/{rest}", ".."),
    ]
}

#[test]
fn every_spelling_of_a_path_through_a_link_to_a_protected_folder_is_judged_where_it_leads() {
    let (_dir, home) = linked_home();
    let _home = super::enter_home(home.clone());
    let root = format!("{home}/scfixture/app");
    let _ctx = enter(PathCtx { cwd: Some(root.clone()), root: Some(root), session_id: None });
    for (link, target, rest) in LINKED {
        for path in spellings(&home, link, rest) {
            let placed = place(&path, Some(Use::Write));
            assert_eq!(placed.judged(), format!("{target}/{rest}"), "{path}");
        }
    }
}

proptest::proptest! {
    #[test]
    fn two_spellings_of_one_path_are_placed_alike(which in 0..LINKED.len(), first in 0usize..6, second in 0usize..6) {
        let (_dir, home) = linked_home();
        let _home = super::enter_home(home.clone());
        let root = format!("{home}/scfixture/app");
        let _ctx = enter(PathCtx { cwd: Some(root.clone()), root: Some(root), session_id: None });
        let (link, _, rest) = LINKED[which];
        let all = spellings(&home, link, rest);
        let (a, b) = (place(&all[first], Some(Use::Write)), place(&all[second], Some(Use::Write)));
        proptest::prop_assert_eq!(a.judged(), b.judged());
    }
}

fn verdict_from(cwd: &str, root: &str, command: &str) -> crate::verdict::Verdict {
    crate::command_verdict_in(command, PathCtx { cwd: Some(cwd.to_string()), root: Some(root.to_string()), session_id: None })
}

#[test]
fn a_write_through_a_link_to_a_protected_folder_is_refused_in_every_spelling() {
    let (_dir, home) = linked_home();
    let _home = super::enter_home(home.clone());
    let root = format!("{home}/scfixture/app");
    assert!(verdict_from(&root, &root, "echo x > src/a.rs").is_allowed(), "a write to an ordinary file stays approved");
    assert!(!verdict_from(&root, &root, "echo x > glink/hooks/pre-commit").is_allowed(), "the link to .git is refused as written");
    for (link, _, rest) in LINKED {
        let refused_as_written = !verdict_from(&root, &root, &format!("echo x > {link}/{rest}")).is_allowed();
        for path in spellings(&home, link, rest) {
            for command in [format!("echo x > {path}"), format!("touch {path}"), format!("tee {path}")] {
                let refused = !verdict_from(&root, &root, &command).is_allowed();
                assert!(refused || !refused_as_written, "{command}");
            }
        }
    }
}

#[test]
fn the_same_write_is_refused_from_a_folder_outside_the_project() {
    let (_dir, home) = linked_home();
    let _home = super::enter_home(home.clone());
    let root = format!("{home}/scfixture/app");
    let command = "echo x > ~/scfixture/app/glink/hooks/pre-commit";
    assert!(!verdict_from(&home, &root, command).is_allowed(), "{command}");
}

#[test]
fn a_link_named_with_a_trailing_slash_is_followed_to_where_it_leads() {
    let (_dir, home) = linked_home();
    let _home = super::enter_home(home.clone());
    let root = format!("{home}/scfixture/app");
    let _ctx = enter(PathCtx { cwd: Some(root.clone()), root: Some(root.clone()), session_id: None });
    for (link, target, _) in LINKED {
        for path in [format!("~/scfixture/app/{link}/"), format!("~/scfixture/app/{link}/."), format!("{link}/")] {
            assert_eq!(place(&path, Some(Use::Write)).judged(), target, "{path}");
        }
    }
}

#[test]
fn a_chained_link_and_one_that_leaves_and_returns_are_judged_where_they_end() {
    let (_dir, home) = linked_home();
    let _home = super::enter_home(home.clone());
    let root = format!("{home}/scfixture/app");
    symlink("glink", format!("{root}/chain")).unwrap();
    symlink("../proj/glink", format!("{root}/back")).unwrap();
    let _ctx = enter(PathCtx { cwd: Some(root.clone()), root: Some(root.clone()), session_id: None });
    for link in ["chain", "back"] {
        for path in [format!("{link}/hooks/pre-commit"), format!("~/scfixture/app/{link}/hooks/pre-commit")] {
            assert_eq!(place(&path, Some(Use::Write)).judged(), ".git/hooks/pre-commit", "{path}");
            let command = format!("echo x > {path}");
            assert!(!verdict_from(&root, &root, &command).is_allowed(), "{command}");
        }
    }
}
