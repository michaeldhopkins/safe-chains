//! A command that follows links while it reads (`tar -h`, `rg -L`, `zip`) reads wherever a link in
//! the tree leads, so it is judged as a read of that place, and a tree it cannot list in full is
//! judged as if it held a link to a secret.

use safe_chains::pathctx::PathCtx;
use safe_chains::{Verdict, command_verdict_in};

struct Project {
    _dir: tempfile::TempDir,
    root: String,
}

fn project() -> Project {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("scratch project");
    let root = dir.path().canonicalize().expect("canonical root").to_str().expect("utf-8 root").to_string();
    std::fs::create_dir_all(format!("{root}/src")).expect("project folder");
    std::fs::write(format!("{root}/src/a.txt"), "a").expect("project file");
    Project { _dir: dir, root }
}

fn at(p: &Project, cmd: &str) -> Verdict {
    command_verdict_in(cmd, PathCtx { cwd: Some(p.root.clone()), root: Some(p.root.clone()), session_id: None })
}

const FOLLOWING_READS: [&str; 8] =
    ["tar -chf - .", "rg -L x", "zip -r - .", "7z a out.7z .", "ag -f x", "cp -RL . y", "rsync -aL ./ y", "find -L . -exec cat {} +"];

#[test]
fn a_link_to_a_folder_outside_the_project_is_judged_as_a_read_of_that_folder() {
    let home = std::env::var("HOME").expect("HOME");
    for target in ["/etc", "/", home.as_str()] {
        let p = project();
        std::os::unix::fs::symlink(target, format!("{}/out", p.root)).expect("link to a folder");
        for cmd in FOLLOWING_READS {
            assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved over a link to {target}");
        }
        for cmd in ["tar -cf - .", "rg x", "zip -y -r - ."] {
            assert!(at(&p, cmd).is_allowed(), "{cmd} was refused though it does not follow the link to {target}");
        }
    }
}

#[test]
fn a_link_to_a_folder_inside_the_project_is_still_read() {
    let p = project();
    std::os::unix::fs::symlink("src", format!("{}/in", p.root)).expect("link inside");
    for cmd in FOLLOWING_READS {
        assert!(at(&p, cmd).is_allowed(), "{cmd} was refused over a link inside the project");
    }
}

#[test]
fn a_tree_with_too_many_links_to_list_is_refused_to_a_following_read() {
    let p = project();
    for n in 0..70 {
        std::os::unix::fs::symlink("/etc/hosts", format!("{}/src/l{n}", p.root)).expect("link to an ordinary file");
    }
    std::os::unix::fs::symlink("/etc/ssh/not-a-real-key", format!("{}/src/zz", p.root)).expect("link to a secret");
    for cmd in FOLLOWING_READS {
        assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved over a tree it could not list");
    }
    assert!(at(&p, "tar -cf - .").is_allowed());
}

#[test]
fn a_tree_with_too_many_entries_to_list_is_refused_to_a_following_read() {
    let p = project();
    for d in 0..21 {
        let dir = format!("{}/many/d{d:02}", p.root);
        std::fs::create_dir_all(&dir).expect("folder");
        for f in 0..1000 {
            std::fs::write(format!("{dir}/f{f}"), "").expect("file");
        }
    }
    std::os::unix::fs::symlink("/etc/ssh/not-a-real-key", format!("{}/many/d20/zz", p.root)).expect("link to a secret");
    for cmd in ["tar -chf - .", "rg -L x", "zip -r - .", "find -L . -exec cat {} +", "fd -L . -x cat"] {
        assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved over a tree it could not list");
    }
    assert!(at(&p, "rg x").is_allowed());
}

fn with_secret_link() -> Project {
    let p = project();
    std::os::unix::fs::symlink("/etc/ssh/not-a-real-key", format!("{}/k", p.root)).expect("link to a secret");
    p
}

#[test]
fn a_y_inside_a_zip_option_value_does_not_stop_it_following_links() {
    let p = with_secret_link();
    for cmd in ["zip -r -n.y - .", "zip -rn.y - .", "zip -r -n .y - .", "zip -r -t2020-01-01y - ."] {
        assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved over a link to a secret");
    }
    for cmd in ["zip -r -y - .", "zip -ry - .", "zip -r -n.z -y - .", "zip -r --symlinks - ."] {
        assert!(at(&p, cmd).is_allowed(), "{cmd} was refused though it stores the link");
    }
}
