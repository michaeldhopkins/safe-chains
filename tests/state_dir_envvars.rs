//! Variables that move where a tool keeps the state it trusts on the next run: borg's keys, cache
//! and security records (which hold the repository location and key type it checks), and go-task's
//! checksums (which decide whether a task is skipped as up to date). Pointed outside the project,
//! they ask, as the matching write would.

use safe_chains::pathctx::{PathCtx, enter};

fn approved(command: &str) -> bool {
    let root = env!("CARGO_MANIFEST_DIR").to_string();
    let _ctx = enter(PathCtx { cwd: Some(root.clone()), root: Some(root), session_id: None });
    safe_chains::is_safe_command(command)
}

#[test]
fn a_state_folder_outside_the_project_asks() {
    let borg = ["BORG_SECURITY_DIR", "BORG_CACHE_DIR", "BORG_BASE_DIR", "BORG_CONFIG_DIR", "BORG_KEYS_DIR", "BORG_KEY_FILE"];
    let cases = borg.iter().map(|name| (*name, "borg list ./repo")).chain([("TASK_TEMP_DIR", "task --list")]);
    for (name, command) in cases {
        assert!(approved(command), "precondition: `{command}`");
        for value in ["/tmp/evil", "/private/tmp/x", "../../elsewhere"] {
            let line = format!("{name}={value} {command}");
            assert!(!approved(&line), "`{line}` was approved");
        }
        let inside = format!("{name}={}/target/state {command}", env!("CARGO_MANIFEST_DIR"));
        assert!(approved(&inside), "`{inside}` should stay approved");
    }
}
