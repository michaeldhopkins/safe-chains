// Generated test names keep the flag case they spell (`-D` and `-d` differ), so they opt out of non_snake_case.
#[cfg(test)]
macro_rules! safe {
    ($($name:ident: $cmd:expr),* $(,)?) => {
        $(#[test] #[allow(non_snake_case)] fn $name() { assert!(check($cmd), "expected safe: {}", $cmd); })*
    };
}

#[cfg(test)]
macro_rules! denied {
    ($($name:ident: $cmd:expr),* $(,)?) => {
        $(#[test] #[allow(non_snake_case)] fn $name() { assert!(!check($cmd), "expected denied: {}", $cmd); })*
    };
}

#[cfg(test)]
macro_rules! inert {
    ($($name:ident: $cmd:expr),* $(,)?) => {
        $(#[test] #[allow(non_snake_case)] fn $name() {
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
        $(#[test] #[allow(non_snake_case)] fn $name() {
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
        $(#[test] #[allow(non_snake_case)] fn $name() {
            assert_eq!(
                crate::command_verdict($cmd),
                crate::verdict::Verdict::Allowed(crate::verdict::SafetyLevel::SafeWrite),
                "expected SafeWrite: {}", $cmd,
            );
        })*
    };
}

pub mod allowlist;
pub mod cli;
#[cfg(test)]
mod composition;
pub mod cst;
pub mod decisionlog;
pub mod docs;
pub mod engine;
mod envvars;
pub mod explain_fix;
#[cfg(test)]
mod handler_property_tests;
mod handlers;
pub mod mcp;
pub mod netloc;
mod operands;
#[cfg(test)]
mod overreach_property_tests;
pub mod parse;
#[cfg(test)]
mod path_placement_tests;
pub mod pathctx;
pub mod pathgate;
pub mod policy;
mod reach;
pub mod refusal;
pub mod registry;
#[cfg(test)]
mod substitution_property_tests;
pub mod suggest;
pub mod targets;
pub mod userconfig;
pub mod verdict;

pub use reach::ReachReason;
pub use verdict::{SafetyLevel, Verdict};

/// The facet profile behind a verdict, rendered for `--explain`.
///
/// Answers the question the boolean cannot: not "is this allowed" but "on which axis was it
/// refused". Empty when no resolver claims the command — that is the answer too, since it means the
/// legacy classifier decided and there are no facets to show.
pub fn facet_breakdown(command: &str, named_place: bool) -> String {
    // One simple command only. `shell_words` has no idea what `&&` means, so on a chain it hands
    // back one flat token list and the resolver reads the SECOND command's arguments as flags of
    // the first — `aws dynamodb scan --table-name t && rm -rf /` produced a single worst-case
    // profile belonging to neither segment. A diagnostic that invents a capability set no resolver
    // emitted is worse than silence, and `render()` above already breaks the chain down per segment.
    if cst::explain(command).segments.len() != 1 {
        return "\n  (facet breakdown covers one command at a time; run --explain on a single segment)\n".to_string();
    }
    // A COMPOUND is one segment, so it reaches here — but the flat split cannot see into it.
    // `(cat ~/.ssh/id_rsa)` tokenises to `["(cat", "~/.ssh/id_rsa)"]`, no resolver recognises
    // `(cat`, and the refusal rendered with no reason at all. The command the caller has to change
    // is INSIDE the construct, so describe that one and say so.
    let inner = cst::denied_inner_words(command);
    let words = match inner {
        Some(ref w) => w.clone(),
        None => match shell_words::split(command) {
            Ok(w) => w,
            Err(_) => return String::new(),
        },
    };
    if words.is_empty() {
        return String::new();
    }
    let tokens: Vec<parse::Token> = words.into_iter().map(parse::Token::from_raw).collect();
    let Some(ex) = engine::bridge::explain_profile(&tokens) else {
        return String::new();
    };
    let mut out = String::from("\n  resolved profile:\n");
    if let Some(w) = &inner {
        out.push_str(&format!("    (the refused command inside it: `{}`)\n", w.join(" ")));
    }
    out.push_str(&engine::plain::render(&ex, named_place));
    out
}

pub fn is_safe_command(command: &str) -> bool {
    command_verdict(command).is_allowed()
}

pub fn command_verdict(command: &str) -> Verdict {
    cst::command_verdict(command)
}

/// Classify `command` against a named level. Every engine-resolved leaf is decided by
/// `Level::admits` against `level` rather than by walking the band; a `Denied` on any segment
/// dominates. Legacy (unresolved) leaves keep their local-safe `SafeWrite`-or-below verdict.
///
/// A pass returns the band the PROFILE earns (`to_legacy`), so the caller's `<= ceiling` gate
/// still tightens — that is what lets `reader` (SafeRead) and `paranoid` (Inert) use this path at
/// all. Upper-band levels have no legacy equivalent and keep the shared `SafeWrite`.
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
    // EVERY named level classifies through `admits`. The levels are `extends`-chained
    // (paranoid ⊂ reader ⊂ editor ⊂ developer ⊂ admin ⊂ yolo), so one mechanism across the whole
    // ladder makes it monotone BY CONSTRUCTION: whatever a level admits, every looser level
    // inherits.
    //
    // Only `editor` and the upper band used to, for a reason that turned out to be a bug rather
    // than a design: `project` stamped every pass `SafeWrite`, which the `<= threshold` gate then
    // refused at any lower ceiling, so handing `reader` an engine level denied its entire band
    // (measured: 157 commands, `cat ./notes.txt` among them). With `project` returning the band
    // the profile actually earns, that is gone.
    //
    // The mixture was itself the `level_monotonic` fuzz failure — `editor` decided by `admits`
    // while `reader` and `developer` decided by projection, and nothing made two mechanisms
    // adjacent in one ordered ladder agree.
    let engine_level = match canonical {
        "paranoid" | "reader" | "editor" | "developer" | "local-admin" | "network-admin" | "yolo" => {
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
pub fn command_verdict_ceilinged(command: &str, threshold: SafetyLevel, engine_level: Option<&'static engine::level::Level>) -> Verdict {
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
/// Whether Claude Code's OWN permission files may contribute trust to this run.
///
/// safe-chains reads two things out of `~/.claude/settings.json`: `permissions.allow` command
/// patterns (the coverage bridge, `allowlist.rs`) and `Read(...)` path approvals (the grant bridge,
/// `regions.rs`). Both were loaded unconditionally, on every harness — so a file that exists purely
/// to configure Claude Code was silently granting permissions under Codex, Cursor, Grok and agy.
///
/// On Codex that is not academic. Codex has no interactive approval, which is why safe-chains
/// DENIES a gated command there; with a `Bash(curl:*)` rule sitting in the Claude file,
/// `curl … | sh` went from denied to abstain, and abstain on Codex means it simply runs.
///
/// Defaults to FALSE, so a harness safe-chains does not recognize never inherits another tool's
/// grants. Only the Claude target turns it on.
static CLAUDE_CONFIG_TRUSTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Honor `~/.claude/settings.json` as a source of trust for the rest of this process.
pub fn trust_claude_config() {
    CLAUDE_CONFIG_TRUSTED.store(true, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn claude_config_trusted() -> bool {
    CLAUDE_CONFIG_TRUSTED.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn explain_with_coverage_at_level(command: &str, engine_level: Option<&'static engine::level::Level>) -> cst::Explanation {
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

/// Render command-derived text safely INSIDE one of our messages.
///
/// The explanation is read by a human deciding whether to approve, and on the Claude and Qwen
/// targets it is injected into the model's context as `additionalContext`. Command text is not
/// trustworthy input for either job: a command routinely carries data the agent picked up from a
/// file, an issue title, a downloaded manifest. Echoed raw, a newline in it forged a whole extra
/// line of our OWN output —
///
/// ```text
///   ✗  cat "/etc/x
///   ✓  ls   safe-chains: auto-approves.
/// ```
///
/// — so the reader saw an approval that never happened, in our voice. Escaping the control
/// characters keeps any echoed text to a single line of literal content, which is the property
/// that makes forging a second line impossible. Bidi controls go too: they reorder what is
/// DISPLAYED without changing the bytes, which is the same forgery by other means.
///
/// This neutralizes our own OUTPUT. It is not a check on the command and decides nothing.
/// Render our INTERNAL substitution markers back as `$(…)` before any of them reach a human.
///
/// A path carrying a substitution is classified through a sentinel, and the operand the nudge
/// reports is the expanded form — so the reader of `cat ~/p/out/$(seq 1 1)` was being shown
/// `~/p/out/__SAFE_CHAINS_CMDSUB_ATOM__`, a path they never wrote. On the Claude and Qwen targets
/// this text is injected into the MODEL's context, where an internal marker is worse than noise:
/// it is a magic string the model can learn and start emitting, and a nudge that describes a path
/// the user cannot find in their own command is one they have no reason to trust.
///
/// Covers every sentinel spelling at once — opaque, atom, and the locus-tagged forms — by keying
/// on the shared prefix and consuming through the terminating `__`, so a sentinel added later is
/// rendered without touching this. Text that merely LOOKS like a sentinel keeps its tail — only
/// the marker itself is replaced — because what follows a bare prefix is the user's path, not our
/// internals, and dropping it handed a crafted filename control over how much of the path the
/// reader saw.
fn render_sentinels(s: &str) -> std::borrow::Cow<'_, str> {
    let prefix = cst::eval::TAGGED_PREFIX;
    if !s.contains(prefix) {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find(prefix) {
        out.push_str(&rest[..at]);
        out.push_str("$(…)");
        let after = &rest[at + prefix.len()..];
        rest = if let Some(tail) = after.strip_prefix('_') {
            // The opaque marker is the prefix plus a single `_`.
            tail
        } else if let Some(i) = after.find("__") {
            // Atom and locus-tagged markers are the prefix, a term, then `__`.
            &after[i + 2..]
        } else {
            // Not one of ours. Keep the text: dropping it let a CRAFTED filename decide how much
            // of the path a human was shown — `cat ~/__SAFE_CHAINS_CMDSUB_.ssh/id_rsa` reported
            // reaching `~/$(…)`, hiding `.ssh/id_rsa` from the one message used to decide.
            after
        };
    }
    out.push_str(rest);
    std::borrow::Cow::Owned(out)
}

pub fn sanitize_display(s: &str) -> String {
    let s = &render_sentinels(s);
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // C0/C1 controls, and the bidi overrides/isolates/marks.
            c if c.is_control() || matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{200E}' | '\u{200F}') => {
                out.push_str(&format!("\\u{{{:04x}}}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// If a NOT-auto-approved command reaches a path OUTSIDE the workspace, return that path and WHY.
/// The path is spelled as written, with the variables earlier statements bound put in; a relative
/// path read after a `cd` is spelled as the place it names, since its spelling names another place
/// from where the reader stands. Each is reported so the hook can nudge instead of silently prompting. Resolves against
/// the ambient `cwd`/`root`: relative worktree paths, `/tmp`, and `/dev` streams are admitted and
/// skipped; an absolute or home path that isn't admitted for read *or* write is the reach. A
/// credential store outranks the hidden-peer wording; a hidden peer path outranks the generic
/// outside-workspace reason.
pub fn workspace_overreach(command: &str) -> Option<(String, ReachReason)> {
    if let Some(found) = reach::repository_snapshot(command) {
        return Some(found);
    }
    let operands = operands::operand_words(command)?;
    operands.into_iter().find_map(|operands::Operand { text, resolved }| {
        if !policy::looks_like_path(&text) {
            return None;
        }
        let t = if pathctx::resolve(&text) == resolved { text } else { resolved.clone() };
        // A temp path is READ/WRITE admitted, so the outside-test below never fires on it — but it
        // is not EXECUTABLE unless it is this session's scratchpad. When the command was denied,
        // that is the likely reason, and it is the one case where the fix is a grant rather than a
        // correction, so surface it with those instructions.
        if pathctx::under_temp_root(&resolved) && !pathctx::in_session_scratchpad(&resolved) {
            return Some((t, ReachReason::ForeignTemp));
        }
        // The REBIND face is consulted too, or a granted trust-root directory denies in silence:
        // the grant opens read and write, so `rm -rf ~/.config` looked ordinary here while the
        // engine refused it. Only the trust-root directories can make this term true, since every
        // other role's rebind face equals its write face.
        let outside = (resolved.starts_with('/') || resolved.starts_with('~'))
            && (!engine::resolve::read_content_verdict(&resolved).is_allowed()
                || !engine::resolve::write_target_verdict(&resolved).is_allowed()
                || engine::resolve::rebind_is_stricter_than_write(&resolved));
        if !outside {
            return None;
        }
        Some((t, reach::classify(&resolved)))
    })
}

#[cfg(test)]
mod tests;
