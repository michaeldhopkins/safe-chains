//! `7z` reads more than its operands while it adds: names listed in a file (`@list`, `-i@list`),
//! and paths its include switches name (`-i!path`, `-ir!path`). Each is judged as a read.

use safe_chains::pathctx::PathCtx;
use safe_chains::{Verdict, command_verdict_in};

fn at(root: &str, cmd: &str) -> Verdict {
    command_verdict_in(cmd, PathCtx { cwd: Some(root.to_string()), root: Some(root.to_string()), session_id: None })
}

#[test]
fn names_7z_reads_from_a_list_or_an_include_switch_are_judged() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("scratch project");
    let root = dir.path().canonicalize().expect("root").to_str().expect("utf-8").to_string();
    std::fs::create_dir_all(format!("{root}/src")).expect("folder");
    std::fs::write(format!("{root}/src/a.rs"), "").expect("file");
    std::fs::write(format!("{root}/list.txt"), "/etc/ssh/not-a-real-key\n").expect("list");
    for cmd in [
        "7z a out.7z @list.txt", "7z a out.7z -i@list.txt", "7z a out.7z -ir@list.txt", "7z a out.7z -i!/etc/ssh/not-a-real-key",
        "7z a out.7z -ir!/etc/ssh/*", "7z a out.7z -ir0!/etc/ssh/x", "7z a out.7z -i!../../etc/ssh/x", "7za a out.7z @list.txt",
        "7zz a out.7z -I!/etc/ssh/x", "7z a -snl out.7z -i!/etc/ssh/x", "7z u out.7z -ir-!~/.ssh/id_ed25519", "7z x -ai@list.txt",
        "7z l -ai!/etc/ssh/x", "7z a out.7z -i!$HOME/.ssh/x",
    ] {
        assert!(!at(&root, cmd).is_allowed(), "{cmd} was approved");
    }
    for cmd in ["7z a out.7z src", "7z a out.7z -i!src/*.rs", "7z a out.7z -x@list.txt src", "7z a out.7z -xr!*.o src", "7z x a.7z -i!x"] {
        assert!(at(&root, cmd).is_allowed(), "{cmd} was refused");
    }
}
