//! Once CDPATH may be set, `cd NAME` can land in any directory a CDPATH entry holds a NAME in, so
//! the working directory after it is unknown and a later write there is not approved. A target
//! starting with `/`, `~`, `.` or `..` does not search CDPATH and is tracked as before.

fn approved_in_project(command: &str) -> bool {
    use safe_chains::pathctx::{PathCtx, enter};
    let project = format!("{}/projects/cdpathprobe", std::env::var("HOME").expect("HOME"));
    let _ctx = enter(PathCtx { cwd: Some(project.clone()), root: Some(project), session_id: None });
    safe_chains::is_safe_command(command)
}

const SETTINGS: &[&str] = &[
    "CDPATH=/elsewhere cd {target} && {body}",
    "CDPATH=/elsewhere; cd {target} && {body}",
    "export CDPATH=/elsewhere; cd {target} && {body}",
    "export CDPATH=/elsewhere && cd {target} && {body}",
    "export -n CDPATH=/elsewhere; cd {target} && {body}",
    "cdpath=/elsewhere; cd {target} && {body}",
    "cdpath=/elsewhere cd {target} && {body}",
    "export cdpath=/elsewhere; cd {target} && {body}",
];

const BODIES: &[&str] = &["jj new", "touch a", "echo x > out.txt"];

#[test]
fn a_relative_cd_under_cdpath_leaves_the_working_directory_unknown() {
    for setting in SETTINGS {
        for body in BODIES {
            let command = setting.replace("{target}", "sub").replace("{body}", body);
            assert!(!approved_in_project(&command), "{command}");
        }
    }
}

#[test]
fn an_anchored_cd_under_cdpath_is_tracked_as_before() {
    for setting in SETTINGS {
        for body in BODIES {
            for target in ["./sub", "../cdpathprobe/sub"] {
                let command = setting.replace("{target}", target).replace("{body}", body);
                assert!(approved_in_project(&command), "{command}");
            }
        }
    }
    for body in BODIES {
        assert!(approved_in_project(&format!("cd sub && {body}")), "without CDPATH a relative cd is tracked");
    }
}

#[test]
fn a_cdpath_in_the_environment_safe_chains_runs_in_counts_as_set() {
    let base = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("tempdir");
    let home = base.path().join("home");
    let work = base.path().join("work");
    std::fs::create_dir_all(&home).expect("mkdir home");
    std::fs::create_dir_all(&work).expect("mkdir work");
    let work = work.to_str().expect("utf-8 path");
    let approves = |cdpath: Option<&str>, command: &str| {
        let mut run = std::process::Command::new(env!("CARGO_BIN_EXE_safe-chains"));
        run.args(["--cwd", work, "--root", work, command])
            .env_clear()
            .env("HOME", &home)
            .env("PATH", "/usr/bin:/bin");
        if let Some(value) = cdpath {
            run.env("CDPATH", value);
        }
        run.stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("run safe-chains")
            .code()
            == Some(0)
    };
    for body in BODIES {
        let command = format!("cd sub && {body}");
        assert!(!approves(Some("/elsewhere"), &command), "{command} with CDPATH inherited");
        assert!(approves(None, &command), "{command} without CDPATH");
        assert!(approves(Some(""), &command), "{command} with an empty CDPATH");
        assert!(approves(Some("/elsewhere"), &format!("cd ./sub && {body}")), "an anchored cd with CDPATH inherited");
    }
}
