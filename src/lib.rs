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
pub mod parse;
pub mod pathctx;
pub mod pathgate;
pub mod policy;
pub mod registry;
pub mod suggest;
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

/// Resolve a level NAME to its `(3-band ceiling, engine level for admits)`, or `None` for an unknown
/// name. The ceiling gates the projected verdict; the engine level (when present) classifies per-level
/// via `admits`, exposing distinctions the 3-band projection flattens — `editor` (no destroy, no
/// sibling write) vs `developer`, and the upper band (git push, bulk-object-read, sudo). `paranoid`/
/// `reader` are pure ceilings (their read/inert bands need no `admits`), and `developer` IS the default
/// band, so those carry no engine level. Legacy aliases (`safe-write`) canonicalize first.
pub fn level_ceiling(name: &str) -> Option<(SafetyLevel, Option<&'static engine::level::Level>)> {
    let (ceiling, legacy_of) = verdict::SafetyLevel::resolve_threshold(name)?;
    let canonical = legacy_of.unwrap_or(name);
    // Levels whose rule the 3-band projection can't express classify per-level via `admits`:
    // `editor` (no destroy, no sibling write — distinct from developer) and the UPPER band (git push,
    // bulk-object-read, sudo — above the band). `paranoid`/`reader` are pure ceilings (their
    // inert/read bands need no `admits`; the `<= threshold` gate tightens), and `developer` IS the
    // default band — those carry no engine level.
    let engine_level = match canonical {
        "editor" | "local-admin" | "network-admin" | "yolo" => {
            engine::authoring::default_levels().iter().find(|l| l.name == canonical)
        }
        _ => None,
    };
    Some((ceiling, engine_level))
}

/// The ceilinged verdict: classify `command` at `(threshold, engine_level)`, gating the projected
/// level `<= threshold`. The single seam both the CLI (`--level`) and the hook (configured `level`)
/// funnel through. `engine_level = Some` classifies via `Level::admits` (the fine per-level model);
/// `None` uses the 3-band projection. Either way the result is gated to `threshold`, so a legacy leaf
/// that bypasses the engine (a redirect write → `SafeWrite`) is still held under a lower ceiling.
pub fn command_verdict_ceilinged(
    command: &str,
    threshold: SafetyLevel,
    engine_level: Option<&'static engine::level::Level>,
) -> Verdict {
    let verdict = match engine_level {
        Some(level) => command_verdict_at_level(command, level),
        None => command_verdict(command),
    };
    match verdict {
        Verdict::Allowed(level) if level <= threshold => Verdict::Allowed(level),
        _ => Verdict::Denied,
    }
}

/// The coverage-fallback explanation (built-in classifier + the user's `permissions.allow` patterns),
/// computed UNDER the configured engine level so a covered command honors that level's rule — a
/// worktree destroy an `editor` plan forbids classifies as denied here too, not re-admitted. `None`
/// engine level → the plain 3-band coverage (paranoid/reader/default). The caller still gates the
/// result's `overall <= threshold`; running under the level closes the last path a lower plan's
/// tighter rule could leak through.
pub fn explain_with_coverage_at_level(
    command: &str,
    engine_level: Option<&'static engine::level::Level>,
) -> cst::Explanation {
    let patterns = allowlist::Matcher::load();
    let _guard = engine_level.map(engine::bridge::enter_eval_level);
    cst::explain_with_coverage(command, &patterns)
}

/// The auto-approve ceiling the HOOK evaluates at, from the write-protected user config
/// (`~/.config/safe-chains.toml`, `level = "…"`). No config, or an unknown name → the default
/// `developer` band (`SafeWrite`, no engine level) — fail-safe. Honored ONLY from the user config,
/// never a repo `.safe-chains.toml`; the file is write-denied, so an agent cannot set its own ceiling.
pub fn configured_hook_ceiling() -> (SafetyLevel, Option<&'static engine::level::Level>) {
    registry::user_config_level()
        .and_then(|name| level_ceiling(&name))
        .unwrap_or((SafetyLevel::SafeWrite, None))
}

/// Classify `command` with the harness-supplied directory context installed (HP-19), so
/// relative paths resolve against the real `cwd`/`root`. `command_verdict(cmd)` is the
/// no-context form (`PathCtx::default()`), preserving every existing caller.
pub fn command_verdict_in(command: &str, ctx: pathctx::PathCtx) -> Verdict {
    let _guard = pathctx::enter(ctx);
    cst::command_verdict(command)
}

/// Why a not-auto-approved command's path reach was flagged — so the nudge can explain the actual
/// reason instead of a one-size-fits-all "outside the working directory". A peer's hidden file and a
/// path genuinely above cwd both deny, but the remedy differs, and conflating them is what reads as
/// "directory parsing is broken".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReachReason {
    /// A known credential store (`.ssh`, `.aws`, keychain…).
    Credential,
    /// A HIDDEN file inside a co-located peer project — the peer's ordinary source is readable as
    /// `adjacent`, but its dotfiles/dotdirs are shielded.
    HiddenPeer,
    /// Genuinely above/outside the working directory.
    OutsideWorkspace,
}

impl ReachReason {
    /// The self-contained nudge body ("it reaches `X`, …") including the reason-appropriate remedy.
    /// Callers add their own framing (block / please-confirm) and the docs link.
    pub fn message(self, path: &str) -> String {
        match self {
            ReachReason::Credential => format!(
                "it reaches `{path}`, a credential store the agent should almost certainly not touch. \
                 If this was not intended, stop it"
            ),
            ReachReason::HiddenPeer => format!(
                "it reaches `{path}`, a HIDDEN file inside a co-located peer project. The peer's \
                 ordinary source is readable, but its hidden files (`.env`, `.git`, `.aws`, …) are \
                 shielded — this is a deliberate guard, not a path error. To reach it, grant that \
                 path in ~/.config/safe-chains.toml, or run the agent from the peer's parent \
                 directory so the peer counts as in-workspace"
            ),
            ReachReason::OutsideWorkspace => match pathctx::cwd() {
                Some(cwd) => format!(
                    "it reaches `{path}`, outside the working directory `{cwd}`. If the agent is \
                     running from the wrong directory — an easy thing to forget — relaunch it where \
                     you meant to be; to allow it from here, grant that path in \
                     ~/.config/safe-chains.toml"
                ),
                None => format!(
                    "it reaches `{path}`, outside the working directory. To allow it, grant that \
                     path in ~/.config/safe-chains.toml"
                ),
            },
        }
    }
}

/// If a NOT-auto-approved command reaches a path OUTSIDE the workspace, return that path (its
/// original spelling) and WHY, so the hook can nudge instead of silently prompting. Resolves against
/// the ambient `cwd`/`root`: relative worktree paths, `/tmp`, and `/dev` streams are admitted and
/// skipped; an absolute or home path that isn't admitted for read *or* write is the reach. A
/// credential store outranks the hidden-peer wording; a hidden peer path outranks the generic
/// outside-workspace reason.
pub fn workspace_overreach(command: &str) -> Option<(String, ReachReason)> {
    let tokens = shell_words::split(command).ok()?;
    tokens.into_iter().find_map(|t| {
        if !policy::looks_like_path(&t) {
            return None;
        }
        let resolved = pathctx::resolve(&t).into_owned();
        let outside = (resolved.starts_with('/') || resolved.starts_with('~'))
            && (!engine::resolve::read_content_verdict(&resolved).is_allowed()
                || !engine::resolve::write_target_verdict(&resolved).is_allowed());
        if !outside {
            return None;
        }
        let reason = if engine::resolve::reads_secret(&resolved) {
            ReachReason::Credential
        } else if engine::resolve::hidden_peer_reach(&t) {
            ReachReason::HiddenPeer
        } else {
            ReachReason::OutsideWorkspace
        };
        Some((t, reason))
    })
}

#[cfg(test)]
mod tests;
