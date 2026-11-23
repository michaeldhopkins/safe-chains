//! `$(pwd)` prints the folder the command runs in, so a path built on it is that folder's path, and a
//! link below it is followed where the kernel follows it, as it is for the same path written relative.

#[path = "support/isolate_home.rs"]
mod isolate_home;

use safe_chains::pathctx::PathCtx;
use safe_chains::{Verdict, command_verdict_in};

fn at(root: &str, command: &str) -> Verdict {
    command_verdict_in(command, PathCtx { cwd: Some(root.to_string()), root: Some(root.to_string()), session_id: None })
}

#[test]
fn a_path_built_on_pwd_is_judged_where_its_links_lead() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("scratch project");
    let root = dir.path().canonicalize().expect("canonical root").to_str().expect("utf-8 root").to_string();
    std::fs::create_dir_all(format!("{root}/sub")).expect("sub");
    std::fs::write(format!("{root}/README.md"), "read me").expect("readme");
    let home = isolate_home::home().to_str().expect("utf-8 home").to_string();
    std::os::unix::fs::symlink(format!("{home}/.ssh"), format!("{root}/leak")).expect("link to .ssh");
    std::os::unix::fs::symlink(format!("{home}/.ssh/id_rsa"), format!("{root}/sub/key")).expect("link to a key");

    let mut approved = Vec::new();
    for command in [
        "cat $(pwd)/leak/id_rsa",
        "cat \"$(pwd)/leak/id_rsa\"",
        "cat $(pwd)/sub/key",
        "cat $(pwd -P)/sub/key",
        "cat $(gpwd)/sub/key",
        "echo k > $(pwd)/leak/authorized_keys",
        "echo k >> \"$(pwd)/leak/authorized_keys\"",
        "cp sub/../README.md $(pwd)/leak/authorized_keys",
        "cd sub && cat $(pwd)/key",
        "cd leak && cat $(pwd)/id_rsa",
        "grep -r x $(pwd)/leak",
        "rg x $(pwd)/leak",
    ] {
        if at(&root, command).is_allowed() {
            approved.push(command);
        }
    }
    assert!(approved.is_empty(), "a path on $(pwd) reached a credential store:\n{}", approved.join("\n"));

    let mut refused = Vec::new();
    for command in ["cat $(pwd)/README.md", "grep -rn foo $(pwd)", "ls $(pwd)/sub", "cd sub && ls $(pwd)", "echo hi > $(pwd)/out.txt"] {
        if !at(&root, command).is_allowed() {
            refused.push(command);
        }
    }
    assert!(refused.is_empty(), "an ordinary path on $(pwd) was refused:\n{}", refused.join("\n"));
}
