//! Property tests for `workspace_overreach`, the reason a denied command is given.

use proptest::prelude::*;

use crate::pathctx::PathCtx;
use crate::{ReachReason, workspace_overreach};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// A temp path is reported as a foreign temp exactly when it is not this session's scratchpad.
    ///
    /// The same file under every temp-root spelling, once in an anonymous directory and once under
    /// the session id: the first must be named `ForeignTemp` (its remedy is a grant), the second
    /// must not be reported at all, since the scratchpad is admitted. Before this, inverting the
    /// scratchpad test in `workspace_overreach` left every test green.
    #[test]
    fn only_a_foreign_temp_path_is_reported_as_foreign_temp(
        root in proptest::sample::select(vec!["/tmp", "/private/tmp", "/var/tmp", "/private/var/tmp"]),
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
}
