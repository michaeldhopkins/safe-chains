use proptest::prelude::*;

use crate::pathctx::{PathCtx, enter};

const SID: &str = "abcdef12-3456-7890-abcd-ef1234567890";
const ROOT: &str = "/work/u/proj";

fn scratchpad() -> String {
    format!("/private/tmp/claude-501/-work-u-proj/{SID}/scratchpad")
}

fn in_project(cmd: &str) -> bool {
    in_folder(ROOT, ROOT, cmd)
}

fn in_folder(cwd: &str, root: &str, cmd: &str) -> bool {
    let _g = enter(PathCtx { cwd: Some(cwd.into()), root: Some(root.into()), session_id: Some(SID.into()) });
    crate::command_verdict(cmd).is_allowed()
}

fn home_root() -> (String, String) {
    let home = std::env::var("HOME").expect("the sibling and parent cases need HOME");
    assert!(home.starts_with('/') && home.len() > 1, "HOME must be an absolute directory: {home}");
    let parent = format!("{home}/projects");
    (format!("{parent}/proj"), parent)
}

#[test]
fn a_config_location_in_a_normal_folder_is_allowed() {
    let home = std::env::var("HOME").expect("the home cases need HOME");
    for cmd in [
        format!("HOME={home} git log"),
        format!("HOME={home}/ git log"),
        format!("HOME={home} jj log"),
        format!("XDG_CONFIG_HOME={home}/.config git log"),
        format!("XDG_CONFIG_HOME={home}/.config jj log"),
        "HOME=/work/u/proj git log".to_string(),
        "HOME=/work/u/proj/fixtures/home jj log".to_string(),
        "HOME= git log".to_string(),
        "HOME=~ git log".to_string(),
        "XDG_CONFIG_HOME= jj log".to_string(),
        "GIT_CONFIG_GLOBAL=/work/u/proj/gitconfig git log".to_string(),
        "JJ_CONFIG=/work/u/proj/config.toml jj log".to_string(),
        "GIT_DIR=/work/u/proj/.git git log".to_string(),
        "GIT_EXEC_PATH= git fetch".to_string(),
    ] {
        assert!(in_project(&cmd), "should be allowed: {cmd}");
    }
}

#[test]
fn a_config_location_in_a_scratch_folder_is_refused() {
    let s = scratchpad();
    let mut cmds: Vec<String> = [
        "HOME=/tmp/x git log",
        "HOME=/tmp git log",
        "HOME=/tmp/ git log",
        "HOME=/private/tmp/x git log",
        "HOME=/private/tmp git log",
        "HOME=/var/tmp/x jj log",
        "HOME=/private/var/tmp/x jj log",
        "XDG_CONFIG_HOME=/tmp/x jj log",
        "XDG_CONFIG_HOME=/private/tmp/x git log",
        "HOME=$TMPDIR/x git log",
        "HOME=$TMPDIR git log",
        "HOME=$(mktemp -d) git log",
        "HOME=/var/folders/ab/cd/T/x git log",
        "HOME=/private/var/folders/ab/cd/T/x git log",
        "GIT_CONFIG_GLOBAL=/tmp/g git log",
        "JJ_CONFIG=/work/u/proj/a.toml:/tmp/b.toml jj log",
        "export HOME=/tmp/x",
        "env HOME=/tmp/x git log",
        "GIT_EXEC_PATH=/tmp/x git fetch",
    ]
    .iter()
    .map(|c| (*c).to_string())
    .collect();
    for var in ["HOME", "XDG_CONFIG_HOME"] {
        for tool in ["git log", "jj log"] {
            cmds.push(format!("{var}={s} {tool}"));
            cmds.push(format!("{var}={s}/home {tool}"));
        }
    }
    for var in ["GIT_CONFIG_GLOBAL", "GIT_CONFIG_SYSTEM", "GIT_DIR", "GIT_COMMON_DIR", "GIT_EXEC_PATH"] {
        cmds.push(format!("{var}={s}/g git log"));
    }
    cmds.push(format!("JJ_CONFIG={s}/config.toml jj log"));
    cmds.push(format!("GIT_EXEC_PATH={s} git fetch"));
    if let Some(t) = std::env::var("TMPDIR").ok().filter(|t| t.starts_with('/') && t.trim_end_matches('/').len() > 1) {
        cmds.push(format!("HOME={}/x git log", t.trim_end_matches('/')));
    }
    for cmd in &cmds {
        assert!(!in_project(cmd), "should be refused: {cmd}");
    }
}

#[test]
fn a_case_variant_of_a_scratch_folder_is_refused_since_macos_folds_case() {
    let s = scratchpad().replacen("/private/tmp", "/PRIVATE/tmp", 1);
    for cmd in [
        "HOME=/TMP/x git log", "HOME=/Tmp/x git log", "HOME=/PRIVATE/TMP/x git log", "HOME=/private/TMP/x git log",
        "HOME=/VAR/folders/ab/T/x git log", "HOME=/var/Folders/ab/T/x git log", "HOME=/VAR/TMP/x git log", "XDG_CONFIG_HOME=/TMP/x jj log",
        "export HOME=/TMP/x; git log", "GIT_CONFIG_GLOBAL=/TMP/g git log", "JJ_CONFIG=/Tmp/c.toml jj log",
    ] {
        assert!(!in_project(cmd), "should be refused: {cmd}");
    }
    assert!(!in_project(&format!("HOME={s} git log")), "should be refused: {s}");
}

#[test]
fn a_folder_outside_the_project_and_home_is_refused_since_it_may_alias_a_scratch_folder() {
    let home = std::env::var("HOME").expect("the alias cases need HOME");
    for cmd in [
        "HOME=/System/Volumes/Data/private/tmp/x git log".to_string(),
        "HOME=/System/Volumes/Data/tmp/x git log".to_string(),
        "HOME=\"/Volumes/Macintosh HD/tmp/x\" git log".to_string(),
        "HOME=\"/Volumes/Macintosh HD/private/tmp/x\" git log".to_string(),
        "HOME=/.vol/16777231/1234 git log".to_string(),
        "HOME=/opt/x git log".to_string(),
        "XDG_CONFIG_HOME=/etc jj log".to_string(),
        format!("HOME=/System/Volumes/Data{home}/projects/sibling git log"),
        format!("HOME={} git log", home.to_uppercase()),
        format!("HOME={home}/../../tmp/x git log"),
        format!("HOME={home}/./../../private/tmp/x git log"),
    ] {
        assert!(!in_project(&cmd), "should be refused: {cmd}");
    }
}

#[test]
fn a_relative_location_is_refused_since_the_program_may_resolve_it_elsewhere() {
    let s = scratchpad();
    for cmd in [
        "HOME=. git log", "HOME=./fixtures/home jj log", "HOME=.. git log", "XDG_CONFIG_HOME=cfg jj log", "GIT_DIR=.git git log",
        "GIT_CONFIG_GLOBAL=./c git log", "JJ_CONFIG=config.toml jj log", "GIT_EXEC_PATH=bin git fetch",
    ] {
        assert!(!in_project(cmd), "should be refused: {cmd}");
        let moved = cmd.replacen(" git ", &format!(" git -C {s} "), 1).replacen(" jj ", &format!(" jj -R {s} "), 1);
        assert!(!in_project(&moved), "should be refused: {moved}");
    }
    let _g = enter(PathCtx::default());
    assert!(!crate::command_verdict("HOME=x git log").is_allowed());
    assert!(crate::command_verdict("HOME=~ git log").is_allowed());
}

#[test]
fn a_config_location_in_a_sibling_project_or_the_projects_parent_is_refused() {
    let (root, parent) = home_root();
    for cmd in [
        format!("HOME={parent}/sibling git log"),
        format!("XDG_CONFIG_HOME={parent}/sibling/cfg jj log"),
        format!("HOME={parent} git log"),
        format!("HOME={parent}/ jj log"),
        format!("XDG_CONFIG_HOME={parent} git log"),
        format!("GIT_CONFIG_GLOBAL={parent}/.gitconfig git log"),
        "HOME=~/projects git log".to_string(),
        "HOME=~/projects/sibling git log".to_string(),
        format!("HOME={}/sibling git log", parent.to_uppercase()),
        format!("HOME={parent}/../projects/sibling git log"),
        format!("HOME={}//sibling git log", parent.replace('/', "//")),
        format!("HOME=/System/Volumes/Data{parent}/sibling git log"),
        format!("HOME={}/sibling git log", parent.to_lowercase()),
        format!("HOME={} git log", parent.to_uppercase()),
    ] {
        assert!(!in_folder(&root, &root, &cmd), "should be refused: {cmd}");
    }
    for cmd in ["HOME=~ git log".to_string(), format!("HOME={root} jj log"), format!("HOME={root}/fixtures git log")] {
        assert!(in_folder(&root, &root, &cmd), "should be allowed: {cmd}");
    }
}

fn segment() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_.-]{1,12}".prop_filter("not a dot segment", |s| s != "." && s != "..")
}

proptest! {
    #[test]
    fn any_config_location_under_a_temp_root_is_refused(
        root in prop::sample::select(vec!["/tmp", "/private/tmp", "/var/tmp", "/private/var/tmp", "/var/folders", "/private/var/folders"]),
        a in segment(),
        b in segment(),
        var in prop::sample::select(vec!["HOME", "XDG_CONFIG_HOME", "GIT_CONFIG_GLOBAL", "JJ_CONFIG", "GIT_DIR", "GIT_EXEC_PATH"]),
        tool in prop::sample::select(vec!["git log", "jj log", "git status", "jj st"]),
    ) {
        let cmd = format!("{var}={root}/{a}/{b} {tool}");
        prop_assert!(!in_project(&cmd), "should be refused: {}", cmd);
        let cmd = format!("{var}={root}/{a} {tool}");
        prop_assert!(!in_project(&cmd), "should be refused: {}", cmd);
    }

    #[test]
    fn any_relative_config_location_is_refused(
        a in segment(),
        b in segment(),
        var in prop::sample::select(vec!["HOME", "XDG_CONFIG_HOME", "GIT_CONFIG_GLOBAL", "JJ_CONFIG", "GIT_DIR", "GIT_EXEC_PATH"]),
    ) {
        let cmd = format!("{var}={a}/{b} git log");
        prop_assert!(!in_project(&cmd), "should be refused: {}", cmd);
    }

    #[test]
    fn a_config_directory_outside_the_scratch_places_is_allowed(
        a in segment(),
        b in segment(),
        var in prop::sample::select(vec!["HOME", "XDG_CONFIG_HOME"]),
        tool in prop::sample::select(vec!["git log", "jj log"]),
    ) {
        let cmd = format!("{var}=/work/u/proj/{a}/{b} {tool}");
        prop_assert!(in_project(&cmd), "should be allowed: {}", cmd);
        let home = std::env::var("HOME").expect("the home case needs HOME");
        let cmd = format!("{var}={home}/.config/{a}/{b} {tool}");
        prop_assert!(in_project(&cmd), "should be allowed: {}", cmd);
    }

    #[test]
    fn the_refusal_is_the_same_with_and_without_the_prefix_spellings(
        a in segment(),
        root in prop::sample::select(vec!["/tmp", "/private/tmp", "/work/u/proj", "/opt", "."]),
    ) {
        let value = format!("{root}/{a}");
        let first = format!("HOME={value} git log");
        let prefix = in_project(&first);
        let other = format!("env HOME={value} git log");
        prop_assert_eq!(prefix, in_project(&other), "{}", other);
        let other = format!("export HOME={value}");
        prop_assert_eq!(prefix, in_project(&other), "{}", other);
    }
}

proptest! {
    #[test]
    fn a_case_variant_of_a_temp_root_is_refused(
        root in prop::sample::select(vec!["/tmp", "/private/tmp", "/var/tmp", "/private/var/folders"]),
        mask in prop::collection::vec(any::<bool>(), 20),
        a in segment(),
    ) {
        let spelled: String = root.chars().zip(mask.iter().cycle()).map(|(c, up)| if *up { c.to_ascii_uppercase() } else { c }).collect();
        let cmd = format!("HOME={spelled}/{a} git log");
        prop_assert!(!in_project(&cmd), "should be refused: {}", cmd);
    }
}
