//! A recursive search given no path searches the folder it runs in. That folder is judged as the
//! tree it reads, so `cd ~/.ssh && rg x`, or a `cd` into a link that leads there, never prints a key.

#[path = "support/isolate_home.rs"]
mod isolate_home;

use safe_chains::pathctx::PathCtx;
use safe_chains::{Verdict, command_verdict_in};

fn searches() -> Vec<String> {
    let gates: toml::Table = toml::from_str(include_str!("../pathgates.toml")).expect("pathgates.toml parses");
    let mut out: Vec<String> = gates["read_tree_after_first"]
        .as_array()
        .expect("read_tree_after_first is a list")
        .iter()
        .map(|name| format!("{} needle", name.as_str().expect("a command name")))
        .collect();
    out.extend(["grep -r needle", "grep -rn needle", "egrep -r needle", "fgrep -r needle"].map(String::from));
    out
}

fn at(root: &str, cwd: &str, command: &str) -> Verdict {
    command_verdict_in(command, PathCtx { cwd: Some(cwd.to_string()), root: Some(root.to_string()), session_id: None })
}

#[test]
fn a_search_with_no_path_is_judged_by_the_folder_it_searches() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("scratch project");
    let root = dir.path().canonicalize().expect("canonical root").to_str().expect("utf-8 root").to_string();
    std::fs::create_dir_all(format!("{root}/src")).expect("src");
    let home = isolate_home::home().to_str().expect("utf-8 home").to_string();
    std::os::unix::fs::symlink(format!("{home}/.ssh"), format!("{root}/leak")).expect("link to .ssh");

    let mut refused_at_home = Vec::new();
    let mut approved = Vec::new();
    for search in searches() {
        if ["rg ", "ag ", "ack ", "grep "].iter().any(|tool| search.starts_with(tool)) && !at(&root, &root, &search).is_allowed() {
            refused_at_home.push(search.clone());
        }
        for line in [format!("cd ~/.ssh && {search}"), format!("cd leak && {search}"), format!("cd src && cd ../leak && {search}")] {
            if at(&root, &root, &line).is_allowed() {
                approved.push(line);
            }
        }
        if at(&root, &format!("{home}/.ssh"), &search).is_allowed() {
            approved.push(format!("{search} (run in ~/.ssh)"));
        }
    }
    assert!(refused_at_home.is_empty(), "a plain search of the project is refused:\n{}", refused_at_home.join("\n"));
    assert!(approved.is_empty(), "a search with no path read a credential store:\n{}", approved.join("\n"));
}
