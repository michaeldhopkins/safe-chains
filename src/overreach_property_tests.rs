//! Property tests for `workspace_overreach`, the reason a denied command is given.

use proptest::prelude::*;

use crate::pathctx::PathCtx;
use crate::{ReachReason, workspace_overreach};

fn host_scratch_roots() -> Vec<&'static str> {
    let mut roots = vec!["/tmp", "/var/tmp"];
    if cfg!(target_os = "macos") {
        roots.extend(["/private/tmp", "/private/var/tmp"]);
    }
    roots
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// A temp path is reported as a foreign temp exactly when it is not this session's scratchpad.
    ///
    /// The same file under every scratch root this platform admits, once in an anonymous directory
    /// and once under the session id: the first must be named `ForeignTemp` (its remedy is a grant),
    /// the second must not be reported at all, since the scratchpad is admitted. Before this,
    /// inverting the scratchpad test in `workspace_overreach` left every test green.
    #[test]
    fn only_a_foreign_temp_path_is_reported_as_foreign_temp(
        root in proptest::sample::select(host_scratch_roots()),
        dir in "[a-z]{1,8}",
        file in "[a-z]{1,8}\\.(sh|txt|py)",
        verb in proptest::sample::select(vec!["cat", "bash", "head", "rm"]),
    ) {
        const SID: &str = "7676dbc5-a265-43b3-a0f8-49666792bd9b";
        let _g = crate::pathctx::enter(PathCtx {
            cwd: Some("/work".into()),
            root: Some("/work".into()),
            session_id: Some(SID.into()),
        });
        let foreign = format!("{root}/{dir}/{file}");
        let reach = workspace_overreach(&format!("{verb} {foreign}"));
        prop_assert_eq!(reach, Some((foreign.clone(), ReachReason::ForeignTemp)), "{}", foreign);

        let scratch = format!("{root}/{dir}/{SID}/scratchpad/{file}");
        let reach = workspace_overreach(&format!("{verb} {scratch}"));
        prop_assert_eq!(reach, None, "{}", scratch);
    }

    /// A path that reaches a command through an earlier statement is reported as that path.
    ///
    /// The same command with the path written in, and with its directory held in a variable bound
    /// by an earlier statement: both reach the same place, so both must name it and give the same
    /// reason. Read one statement at a time, the second named `$P/…`, which is no place at all.
    #[test]
    fn a_path_reached_through_an_earlier_statement_is_reported_as_itself(
        root in proptest::sample::select(vec!["/opt", "/srv", "/Users/someone", "/home/someone", "/tmp", "/etc"]),
        dir in "[a-z]{1,8}",
        file in "[a-z]{1,8}\\.(sh|txt|py)",
        verb in proptest::sample::select(vec!["cat", "bash", "head", "rm", "touch"]),
        join in proptest::sample::select(vec!["; ", " && ", "; ls; "]),
    ) {
        let _g = crate::pathctx::enter(PathCtx {
            cwd: Some("/work".into()),
            root: Some("/work".into()),
            session_id: None,
        });
        let place = format!("{root}/{dir}");
        let path = format!("{place}/{file}");
        let literal = workspace_overreach(&format!("{verb} {path}"));
        prop_assert!(literal.is_some(), "{} {} is not reported at all", verb, path);
        let held = workspace_overreach(&format!("P={place}{join}{verb} $P/{file}"));
        prop_assert_eq!(held, literal, "{}", path);

        let unknown = workspace_overreach(&format!("{verb} $X/{file}"));
        let held_unknown = workspace_overreach(&format!("P=$X{join}{verb} $P/{file}"));
        prop_assert_eq!(held_unknown, unknown, "a variable holding an unknown value named a place");
        let moved_unknown = workspace_overreach(&format!("cd $(foo){join}{verb} ./{file}"));
        let named = moved_unknown.map(|(path, _)| path);
        prop_assert!(!named.as_deref().is_some_and(crate::cst::check::is_opaque_value), "{:?}", named);
    }

    /// `/var/folders` is not a scratch region. A refusal there keeps its own reason, and must not
    /// claim that reading and writing the file is fine.
    #[test]
    fn a_path_under_var_folders_is_not_a_foreign_temp(
        root in prop::sample::select(vec!["/var/folders", "/private/var/folders"]),
        kind in prop::sample::select(vec!["T", "C", "0", "X"]),
        file in "[a-z]{1,8}",
        verb in prop::sample::select(vec!["cat", "bash", "rm"]),
    ) {
        let path = format!("{root}/xx/yy/{kind}/{file}");
        let reach = workspace_overreach(&format!("{verb} {path}"));
        prop_assert_ne!(reach.as_ref().map(|(_, reason)| *reason), Some(ReachReason::ForeignTemp), "{}", path);
        let message = reach.map(|(path, reason)| reason.message(&path)).unwrap_or_default();
        prop_assert!(!message.contains("Reading and writing temp files is fine"), "{}", path);
    }

    #[test]
    fn a_temp_path_whose_loop_var_is_only_half_pinned_is_not_a_foreign_temp(
        root in proptest::sample::select(host_scratch_roots()),
        file in "[a-z]{1,8}\\.(sh|txt|py)",
        verb in proptest::sample::select(vec!["cat", "bash", "head", "rm"]),
    ) {
        let _g = crate::pathctx::enter(PathCtx {
            cwd: Some("/work".into()),
            root: Some("/work".into()),
            session_id: None,
        });
        let _v = crate::pathctx::enter_loop_var(
            "f".into(),
            "__SAFE_CHAINS_CMDSUB__".into(),
            file,
        );
        let path = format!("{root}/$f");
        let reach = workspace_overreach(&format!("{verb} {path}"));
        prop_assert_eq!(reach, Some((path.clone(), ReachReason::Unconfined)), "{}", path);
    }
}

#[test]
fn a_cat_under_var_folders_is_not_explained_as_a_foreign_temp() {
    let path = "/var/folders/xx/yy/T/f";
    for verb in ["cat", "bash", "rm"] {
        let command = format!("{verb} {path}");
        let reach = workspace_overreach(&command);
        let message = reach.as_ref().map(|(path, reason)| reason.message(path)).unwrap_or_default();
        assert!(!message.contains("Reading and writing temp files is fine"), "{command}: {message}");
        assert_eq!(reach.map(|(_, reason)| reason), Some(ReachReason::OutsideWorkspace), "{command}: {message}");
    }
    assert!(crate::is_safe_command(&format!("cat {path}")));
    assert!(!crate::is_safe_command(&format!("bash {path}")));
    assert!(!crate::is_safe_command(&format!("rm {path}")));
}
