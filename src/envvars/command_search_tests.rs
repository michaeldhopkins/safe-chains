use proptest::prelude::*;

use crate::pathctx::{PathCtx, enter};

const ROOT: &str = "/work/u/proj";
const TRUSTED: &[&str] =
    &["$PATH", "${PATH}", "/usr/bin", "/bin", "/usr/local/bin", "/opt/homebrew/bin", "/usr/sbin", "/sbin", "/usr/bin/"];
const FOREIGN: &[&str] = &[
    "/tmp/evil", "/private/tmp/x", "/opt/x/bin", "~/bin", "$HOME/.cargo/bin", "${HOME}/bin", "/Users/someone/Downloads", "/usr/bin/..",
    "/usr/bin/../../tmp", "/usr/local/bin/x", "bin", "./bin", "../bin", ".", "", "/", "/etc", "$(mktemp -d)", "$X",
];
const COMMANDS: &[&str] = &["ls", "git status", "cat ./notes.md", "jq . ./a.json"];

fn allowed(cmd: &str) -> bool {
    let _g = enter(PathCtx { cwd: Some(ROOT.into()), root: Some(ROOT.into()), session_id: None });
    crate::command_verdict(cmd).is_allowed()
}

fn spellings(value: &str, command: &str) -> Vec<String> {
    vec![
        format!("PATH={value} {command}"),
        format!("PATH='{value}' {command}"),
        format!("env PATH={value} {command}"),
        format!("env -i PATH={value} {command}"),
        format!("export PATH={value}; {command}"),
        format!("export PATH=\"{value}\" && {command}"),
        format!("PATH={value}; {command}"),
        format!("declare -x PATH={value}; {command}"),
        format!("typeset PATH={value}; {command}"),
        format!("PATH+=:{value}; {command}"),
        format!("export PATH+=:{value}; {command}"),
        format!("path+={value}; {command}"),
        format!("path={value}; {command}"),
    ]
}

fn joined(elements: &[&str]) -> String {
    elements.join(":")
}

proptest! {
    #[test]
    fn a_foreign_search_folder_anywhere_in_path_is_refused(
        before in prop::collection::vec(prop::sample::select(TRUSTED), 0..3),
        foreign in prop::sample::select(FOREIGN),
        after in prop::collection::vec(prop::sample::select(TRUSTED), 0..3),
        command in prop::sample::select(COMMANDS),
    ) {
        let mut elements = before.clone();
        elements.push(foreign);
        elements.extend(after.iter().copied());
        let value = joined(&elements);
        for cmd in spellings(&value, command) {
            prop_assert!(!allowed(&cmd), "should be refused: {}", cmd);
        }
    }

    #[test]
    fn a_path_of_standard_folders_keeps_the_commands_verdict(
        elements in prop::collection::vec(prop::sample::select(TRUSTED), 1..4),
        command in prop::sample::select(COMMANDS),
    ) {
        let bare = allowed(command);
        prop_assert!(bare, "the bare command must be allowed: {}", command);
        let value = joined(&elements);
        let expands = |c: &String| !value.contains('$') || (c.starts_with("PATH") && !c.starts_with("PATH='"));
        for cmd in spellings(&value, command).into_iter().filter(expands) {
            prop_assert!(allowed(&cmd),"should be allowed: {}", cmd);
        }
    }
}

#[test]
fn an_empty_search_path_is_refused() {
    for cmd in ["PATH= ls", "PATH='' ls", "export PATH=; ls", "env PATH= ls", "PATH=: ls", "PATH=/usr/bin: ls", "PATH=:/usr/bin ls"] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
}

#[test]
fn rebinding_path_without_an_assignment_is_refused() {
    for cmd in [
        "read PATH; ls", "for PATH in /tmp/evil; do ls; done", "declare -n PATH=x; ls", "unset PATH; ls", "unset -v PATH; ls",
        "unset path; ls",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
    assert!(allowed("for PATH in /usr/bin; do ls; done"));
    assert!(allowed("unset FOO; ls"));
}

#[test]
fn a_project_folder_on_the_search_path_is_the_projects_own_code() {
    assert!(allowed(&format!("PATH={ROOT}/bin:$PATH ls")));
    assert!(!allowed("PATH=/work/u/other/bin:$PATH ls"));
}

#[test]
fn a_foreign_folder_between_two_braced_path_elements_is_refused() {
    for cmd in [
        "export PATH=${PATH}:$(mktemp -d):${PATH}; ls",
        "declare -x PATH=${PATH}:$(mktemp -d):${PATH}; ls",
        "typeset -x PATH=${PATH}:$(mktemp -d):${PATH}; ls",
        "export PATH=${PATH}:/tmp/evil:${PATH}; ls",
        "export LD_PRELOAD=${X}:$(mktemp -d)/a.so:${X}; ls",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
    assert!(allowed("export PATH=${PATH}:/opt/homebrew/bin:${PATH}; ls"));
}
