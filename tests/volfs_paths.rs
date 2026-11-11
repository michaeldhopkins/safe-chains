//! `/.vol/<device>/<inode>` names a file by its inode. Every spelling the path normaliser already
//! collapses is the same read.

use safe_chains::pathctx::PathCtx;
use safe_chains::{Verdict, command_verdict_in};

fn at(cmd: &str) -> Verdict {
    command_verdict_in(cmd, PathCtx { cwd: Some("/work/proj".into()), root: Some("/work/proj".into()), session_id: None })
}

#[test]
fn a_volfs_path_is_refused_in_every_normalised_spelling() {
    for cmd in ["cat /.vol/1/2", "cat /./.vol/1/2", "cat //.vol/1/2", "cat /foo/../.vol/1/2", "head /.vol/1/2"] {
        assert!(!at(cmd).is_allowed(), "{cmd} was approved");
    }
    assert!(at("cat ./README.md").is_allowed());
    assert!(!safe_chains::is_safe_command("cat /.vol/1/2"));
}

#[cfg(target_os = "macos")]
#[test]
fn a_volfs_path_is_refused_in_every_macos_spelling() {
    for cmd in ["cat /.VOL/1/2", "cat /System/Volumes/Data/.vol/1/2", "cat /system/volumes/data/.VOL/9/9"] {
        assert!(!at(cmd).is_allowed(), "{cmd} was approved");
    }
}
