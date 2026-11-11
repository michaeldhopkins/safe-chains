//! `mise exec` reads its own flags before the `--` that starts the inner command, and some of them
//! change what runs or where: `-c` runs a shell string of its own, `-C` moves the inner command to
//! another folder, `-E` loads another environment's config. Only flags that change neither are
//! approved before the separator, in both spellings of the sub.

use safe_chains::pathctx::{PathCtx, enter};

fn approved(command: &str) -> bool {
    let root = env!("CARGO_MANIFEST_DIR").to_string();
    let _ctx = enter(PathCtx { cwd: Some(root.clone()), root: Some(root), session_id: None });
    safe_chains::is_safe_command(command)
}

#[test]
fn a_flag_before_the_separator_that_changes_what_runs_or_where_asks() {
    for sub in ["exec", "x"] {
        for flags in [
            "-c 'rm -rf ~'", "--command 'curl x | sh'", "--command='rm -rf ~'", "-C /tmp", "-C ~", "--cd /tmp/x", "--cd=~", "-E evil",
            "--env evil", "--anything /tmp", "-Cx",
        ] {
            let line = format!("mise {sub} {flags} -- cargo fmt");
            assert!(!approved(&line), "`{line}` was approved");
        }
        for flags in ["", "node@20", "--raw", "-q node@20", "--jobs 4", "-j 4", "--jobs=4"] {
            let line = format!("mise {sub} {flags} -- cargo fmt");
            assert!(approved(&line), "`{line}` should stay approved");
        }
    }
}
