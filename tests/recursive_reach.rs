//! A recursive write reaches everything below its operand, so it is judged by the write-frozen
//! metadata (`.git`, `.jj`, `.hg`, `.sl`, `.envrc`) it would walk into, at any depth.

use proptest::prelude::*;
use safe_chains::pathctx::PathCtx;
use safe_chains::{Verdict, command_verdict_in};

struct Project {
    _dir: tempfile::TempDir,
    root: String,
}

fn project(dirs: &[&str]) -> Project {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("scratch project");
    let root = dir.path().canonicalize().expect("canonical root").to_str().expect("utf-8 root").to_string();
    for sub in dirs {
        std::fs::create_dir_all(format!("{root}/{sub}")).expect("project folders");
    }
    std::fs::write(format!("{root}/src/a.txt"), "a").expect("project file");
    Project { _dir: dir, root }
}

fn at(p: &Project, cmd: &str) -> Verdict {
    command_verdict_in(cmd, PathCtx { cwd: Some(p.root.clone()), root: Some(p.root.clone()), session_id: None })
}

const RECURSIVE_WRITES: [&str; 12] = [
    "chmod -R 755 BASE",
    "chmod -R u+x BASE",
    "chflags -R uchg BASE",
    "find BASE -exec chmod +x {} +",
    "find BASE -type f -exec sed -i s/a/b/ {} +",
    "fd -H . BASE -x chmod +x",
    "cp -r BASE backup",
    "rm -rf BASE",
    "rsync -a BASE/ backup",
    "ditto BASE backup",
    "find BASE -print0 | xargs -0 chmod +x",
    "find BASE -type f | while read f; do chmod +x \"$f\"; done",
];

#[test]
fn a_recursive_write_over_the_project_root_is_refused() {
    let p = project(&["src", ".git/hooks"]);
    for cmd in RECURSIVE_WRITES {
        let cmd = cmd.replace("BASE", ".");
        assert!(!at(&p, &cmd).is_allowed(), "{cmd} was approved");
    }
}

#[test]
fn copying_a_tree_that_holds_a_repository_over_the_project_is_refused() {
    let p = project(&["src", ".git/hooks", "other/.git/hooks"]);
    assert!(!at(&p, "cp -r other/. .").is_allowed());
    assert!(!at(&p, "cp -R other dest").is_allowed());
    for cmd in ["rsync -a other/ .", "rsync -r other dest", "ditto other .", "cp -a other/. ."] {
        assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved");
    }
    for cmd in ["cp -r src dest", "rsync -a src/ dest", "ditto src dest"] {
        assert!(at(&p, cmd).is_allowed(), "{cmd} was refused");
    }
}

#[test]
fn a_recursive_write_over_a_plain_folder_still_approves() {
    let p = project(&["src/sub", ".git/hooks"]);
    for cmd in
        ["chmod -R 755 src", "chflags -R nouchg src", "find src -exec chmod +x {} +", "fd . -x chmod +x", "chmod 755 .", "rm -rf src"]
    {
        assert!(at(&p, cmd).is_allowed(), "{cmd} was refused");
    }
}

#[test]
fn a_traversal_that_only_reads_still_approves_over_the_root() {
    let p = project(&["src", ".git/hooks"]);
    for cmd in ["find . -exec cat {} +", "find . -name x -print", "fd -H . -x wc -l", "find . -print0 | xargs -0 wc -l"] {
        assert!(at(&p, cmd).is_allowed(), "{cmd} was refused");
    }
}

#[test]
fn a_tree_too_large_to_list_refuses_writes_and_keeps_reads() {
    let p = project(&["src", "big"]);
    for i in 0..20_001 {
        std::fs::write(format!("{}/big/{i}", p.root), "").expect("file");
    }
    assert!(!at(&p, "chmod -R 755 big").is_allowed());
    assert!(at(&p, "find big -exec cat {} +").is_allowed());
    assert!(at(&p, "rm -rf big").is_allowed());
}

#[test]
fn a_glob_operand_reaches_what_its_matches_hold() {
    let p = project(&["src", "vendor/lib/.git"]);
    assert!(!at(&p, "chmod -R 755 *").is_allowed());
    assert!(at(&p, "chmod -R 755 s*").is_allowed());
}

const FROZEN: [&str; 5] = [".git", ".jj", ".hg", ".sl", ".envrc"];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Wherever a frozen name sits below the operand, every recursive write over it is refused;
    /// the same tree without it is approved.
    #[test]
    fn a_frozen_name_at_any_depth_is_reached(
        depth in proptest::collection::vec("[a-z]{1,6}", 0..4),
        frozen in 0..FROZEN.len(),
        command in 0..RECURSIVE_WRITES.len(),
    ) {
        let parent = std::iter::once("work".to_string()).chain(depth).collect::<Vec<_>>().join("/");
        let p = project(&["src", parent.as_str()]);
        let cmd = RECURSIVE_WRITES[command].replace("BASE", "work");
        prop_assert!(at(&p, &cmd).is_allowed(), "{} refused with nothing frozen below", cmd);
        let name = format!("{}/{parent}/{}", p.root, FROZEN[frozen]);
        if FROZEN[frozen] == ".envrc" {
            std::fs::write(&name, "").expect("frozen file");
        } else {
            std::fs::create_dir_all(&name).expect("frozen folder");
        }
        prop_assert!(!at(&p, &cmd).is_allowed(), "{} approved with {} below", cmd, name);
    }
}

#[test]
fn a_recursive_operand_that_cannot_be_placed_is_refused() {
    let p = project(&["src", "sub/.git"]);
    for cmd in [
        "chmod -R 755 \"$(pwd)\"", "chmod -R 755 \"$(pwd)/src\"", "cp -r \"$(pwd)/sub/.\" dest",
        "for d in . ; do chmod -R 755 \"$d\"; done",
    ] {
        assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved");
    }
    assert!(at(&p, "chmod -R 755 src").is_allowed());
}

#[test]
fn copying_a_metadata_folder_itself_is_refused() {
    let p = project(&["src", "sub/.git/hooks", "d"]);
    for cmd in ["rsync -a sub/.git d/", "rsync -r sub/.git d", "cp -r sub/.git d/"] {
        assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved");
    }
    assert!(at(&p, "rsync -a sub/.git/hooks/ d/").is_allowed());
}
