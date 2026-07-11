#[cfg(test)]
macro_rules! safe {
    ($($name:ident: $cmd:expr),* $(,)?) => {
        $(#[test] fn $name() { assert!(check($cmd), "expected safe: {}", $cmd); })*
    };
}

#[cfg(test)]
macro_rules! denied {
    ($($name:ident: $cmd:expr),* $(,)?) => {
        $(#[test] fn $name() { assert!(!check($cmd), "expected denied: {}", $cmd); })*
    };
}

#[cfg(test)]
macro_rules! inert {
    ($($name:ident: $cmd:expr),* $(,)?) => {
        $(#[test] fn $name() {
            assert_eq!(
                crate::command_verdict($cmd),
                crate::verdict::Verdict::Allowed(crate::verdict::SafetyLevel::Inert),
                "expected Inert: {}", $cmd,
            );
        })*
    };
}

#[cfg(test)]
macro_rules! safe_read {
    ($($name:ident: $cmd:expr),* $(,)?) => {
        $(#[test] fn $name() {
            assert_eq!(
                crate::command_verdict($cmd),
                crate::verdict::Verdict::Allowed(crate::verdict::SafetyLevel::SafeRead),
                "expected SafeRead: {}", $cmd,
            );
        })*
    };
}

#[cfg(test)]
macro_rules! safe_write {
    ($($name:ident: $cmd:expr),* $(,)?) => {
        $(#[test] fn $name() {
            assert_eq!(
                crate::command_verdict($cmd),
                crate::verdict::Verdict::Allowed(crate::verdict::SafetyLevel::SafeWrite),
                "expected SafeWrite: {}", $cmd,
            );
        })*
    };
}

pub mod cli;
#[cfg(test)]
mod composition;
pub mod cst;
#[cfg(test)]
mod handler_property_tests;
pub mod docs;
pub mod engine;
mod handlers;
pub use handlers::all_opencode_patterns;
pub mod parse;
pub mod pathctx;
pub mod pathgate;
pub mod policy;
pub mod registry;
pub mod allowlist;
pub mod targets;
pub mod verdict;

pub use verdict::{SafetyLevel, Verdict};

pub fn is_safe_command(command: &str) -> bool {
    command_verdict(command).is_allowed()
}

pub fn command_verdict(command: &str) -> Verdict {
    cst::command_verdict(command)
}

/// Classify `command` against an UPPER-band level (`local-admin`/`network-admin`/`yolo`), which
/// has no 3-value legacy ceiling. Every engine-resolved leaf is decided by `Level::admits`
/// against `level` instead of the lower-band projection; a `Denied` on any segment dominates.
/// Legacy (unresolved) leaves keep their local-safe `SafeWrite`-or-below verdict, which every
/// upper level admits. The result is `Allowed(SafeWrite)` (accepted by the shared upper ceiling)
/// or `Denied`.
pub fn command_verdict_at_level(command: &str, level: &'static engine::level::Level) -> Verdict {
    let _guard = engine::bridge::enter_eval_level(level);
    cst::command_verdict(command)
}

/// The `&'static Level` for an UPPER-band level name, or `None` for the lower band (which the
/// 3-value ceiling already handles) or an unknown name. The caller passes the CANONICAL name
/// (legacy aliases already resolved).
pub fn upper_level_by_name(name: &str) -> Option<&'static engine::level::Level> {
    if !matches!(name, "local-admin" | "network-admin" | "yolo") {
        return None;
    }
    engine::authoring::default_levels().iter().find(|l| l.name == name)
}

/// Classify `command` with the harness-supplied directory context installed (HP-19), so
/// relative paths resolve against the real `cwd`/`root`. `command_verdict(cmd)` is the
/// no-context form (`PathCtx::default()`), preserving every existing caller.
pub fn command_verdict_in(command: &str, ctx: pathctx::PathCtx) -> Verdict {
    let _guard = pathctx::enter(ctx);
    cst::command_verdict(command)
}

/// If a NOT-auto-approved command reaches a path OUTSIDE the workspace, return that path (its
/// original spelling) so the hook can nudge instead of silently prompting. Resolves against the
/// ambient `cwd`/`root`: relative worktree paths, `/tmp`, and `/dev` streams are admitted and
/// skipped; an absolute or home path that isn't admitted for read *or* write is the reach.
pub fn workspace_overreach(command: &str) -> Option<(String, bool)> {
    let tokens = shell_words::split(command).ok()?;
    tokens.into_iter().find_map(|t| {
        if !policy::looks_like_path(&t) {
            return None;
        }
        let resolved = pathctx::resolve(&t).into_owned();
        let outside = (resolved.starts_with('/') || resolved.starts_with('~'))
            && (!engine::resolve::read_content_verdict(&resolved).is_allowed()
                || !engine::resolve::write_target_verdict(&resolved).is_allowed());
        outside.then(|| (t, engine::resolve::reads_secret(&resolved)))
    })
}

#[cfg(test)]
mod tests;
