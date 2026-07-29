// The generated test names spell the command and its flags verbatim, and FLAG CASE IS MEANINGFUL:
// find takes both `-D` and `-d`, `-P` and `-p`, `-L` and `-l`, and commands like `asn1Decoding` and
// `checkLocalKDC` are camel-case upstream. Lowercasing to satisfy `non_snake_case` would erase the
// distinction the test exists to pin, so the generated items opt out instead.
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

pub mod cli;
#[cfg(test)]
mod composition;
pub mod cst;
#[cfg(test)]
mod handler_property_tests;
pub mod docs;
pub mod engine;
mod envvars;
mod handlers;
pub mod netloc;
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

/// The facet profile behind a verdict, rendered for `--explain`.
///
/// Answers the question the boolean cannot: not "is this allowed" but "on which axis was it
/// refused". Empty when no resolver claims the command — that is the answer too, since it means the
/// legacy classifier decided and there are no facets to show.
pub fn facet_breakdown(command: &str) -> String {
    // One simple command only. `shell_words` has no idea what `&&` means, so on a chain it hands
    // back one flat token list and the resolver reads the SECOND command's arguments as flags of
    // the first — `aws dynamodb scan --table-name t && rm -rf /` produced a single worst-case
    // profile belonging to neither segment. A diagnostic that invents a capability set no resolver
    // emitted is worse than silence, and `render()` above already breaks the chain down per segment.
    if cst::explain(command).segments.len() != 1 {
        return "\n  (facet breakdown covers one command at a time; run --explain on a single segment)\n"
            .to_string();
    }
    let Ok(words) = shell_words::split(command) else {
        return String::new();
    };
    if words.is_empty() {
        return String::new();
    }
    let tokens: Vec<parse::Token> = words.into_iter().map(parse::Token::from_raw).collect();
    let Some(ex) = engine::bridge::explain_profile(&tokens) else {
        return String::new();
    };
    let mut out = String::from("\n  resolved profile:\n");
    for (because, facets) in &ex.capabilities {
        out.push_str(&format!("    · {because}\n"));
        for (name, term) in facets {
            out.push_str(&format!("        {name:<28} {term}\n"));
        }
    }
    match &ex.blocked_by {
        Some((level, mismatch)) => {
            out.push_str(&format!(
                "\n  refused by `{level}` (the most permissive auto-approving level):\n    {mismatch}\n",
            ));
        }
        None => out.push_str("\n  admitted by the auto-approve band.\n"),
    }
    out
}

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
    /// A temp path that is NOT this session's scratchpad. Reading and writing it is fine; RUNNING
    /// code from it is not, because anonymous `/tmp` is where downloaded/foreign code lands. This
    /// is the one reach whose remedy is usually "that IS my working directory" — so the nudge says
    /// how to bless it rather than implying the agent did something wrong.
    ForeignTemp,
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
pub(crate) fn sanitize_display(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // C0/C1 controls, and the bidi overrides/isolates/marks.
            c if c.is_control()
                || matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{200E}' | '\u{200F}') =>
            {
                out.push_str(&format!("\\u{{{:04x}}}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

impl ReachReason {
    /// The self-contained nudge body ("it reaches `X`, …") including the reason-appropriate remedy.
    /// Callers add their own framing (block / please-confirm) and the docs link.
    pub fn message(self, path: &str) -> String {
        let path = &sanitize_display(path);
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
            ReachReason::ForeignTemp => format!(
                "it runs code from `{path}`, a temporary directory that is not this session's \
                 scratchpad. Temp files can be read and written freely, but code there is treated \
                 as FOREIGN (a downloaded script lands in the same place), so running it is not \
                 auto-approved. If this is a working directory you trust, grant it in \
                 ~/.config/safe-chains.toml; a scratchpad the harness reports for this session is \
                 recognized automatically and needs no grant"
            ),
            ReachReason::OutsideWorkspace => match pathctx::cwd().map(|c| sanitize_display(&c)) {
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
    let tokens = operand_words(command)?;
    tokens.into_iter().find_map(|t| {
        if !policy::looks_like_path(&t) {
            return None;
        }
        let resolved = pathctx::resolve(&t).into_owned();
        // A temp path is READ/WRITE admitted, so the outside-test below never fires on it — but it
        // is not EXECUTABLE unless it is this session's scratchpad. When the command was denied,
        // that is the likely reason, and it is the one case where the fix is a grant rather than a
        // correction, so surface it with those instructions.
        if pathctx::under_temp_root(&resolved) && !pathctx::in_session_scratchpad(&resolved) {
            return Some((t, ReachReason::ForeignTemp));
        }
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

/// The words a command actually RUNS with, for explaining a denial.
///
/// This must agree with the parse the verdict came from, so it walks the CST. Splitting the raw
/// string instead (`shell_words::split`) tokenizes text the shell never treats as an argument —
/// above all a heredoc BODY, which is data. `git commit -m "$(cat <<'EOF' … EOF)"` whose message
/// merely MENTIONS `/etc/hosts` was reported as "reaches /etc/hosts", naming a false reason for the
/// denial and advising the reader to grant that path — a config widening the command never needed.
///
/// Falls back to the raw split only when the command does not parse, where a best-effort nudge on
/// approximate tokens still beats none.
fn operand_words(command: &str) -> Option<Vec<String>> {
    let Some(script) = cst::parse(command) else {
        return shell_words::split(command).ok();
    };
    let mut out = Vec::new();
    collect_script_words(&script, &mut out);
    Some(out)
}

/// A word contributes its own expansions AND the words of any command substitution inside it: the
/// inner command runs, so `notacommand $(cat /etc/shadow)` really does read the file, even though
/// `expand()` renders the substitution as an opaque stand-in and hides the path.
fn collect_word(word: &cst::Word, out: &mut Vec<String>) {
    out.extend(word.expand());
    for part in &word.0 {
        collect_part_subs(part, out);
    }
}

/// The words of any command SUBSTITUTION inside a word part, and nothing else — the part's own
/// literal text is the caller's business, because whether it counts as an operand depends on where
/// the word came from (a heredoc body's literal text never does).
fn collect_part_subs(part: &cst::WordPart, out: &mut Vec<String>) {
    use cst::WordPart;
    match part {
        WordPart::CmdSub(script) | WordPart::ProcSub(script) => collect_script_words(script, out),
        WordPart::DQuote(inner) => collect_word(inner, out),
        WordPart::Lit(_)
        | WordPart::Escape(_)
        | WordPart::SQuote(_)
        | WordPart::Backtick(_)
        | WordPart::Arith(_) => {}
    }
}

fn collect_script_words(script: &cst::Script, out: &mut Vec<String>) {
    for stmt in &script.0 {
        for cmd in &stmt.pipeline.commands {
            collect_cmd_words(cmd, out);
        }
    }
}

/// A redirect TARGET is a path the command opens, so it is a reach and must be reported —
/// `notacommand > /etc/passwd` names `/etc/passwd`. A heredoc DELIMITER is not a path at all, and
/// its body never appears in the CST, which is the whole point.
fn collect_redir_words(redirs: &[cst::Redir], out: &mut Vec<String>) {
    use cst::Redir;
    for redir in redirs {
        match redir {
            Redir::Write { target, .. }
            | Redir::Read { target, .. }
            | Redir::ReadWrite { target, .. }
            | Redir::HereStr(target) => collect_word(target, out),
            // Only the body's SUBSTITUTIONS, never its literal text. Behind a bare delimiter a
            // `$(cat /etc/shadow)` in the body really runs, so it is a reach worth naming; the
            // prose around it is data and naming it would state a false reason for the denial.
            Redir::HereDoc { body, .. } => {
                for part in &body.0 {
                    collect_part_subs(part, out);
                }
            }
            Redir::DupFd { .. } => {}
        }
    }
}

fn collect_cmd_words(cmd: &cst::Cmd, out: &mut Vec<String>) {
    use cst::Cmd;
    let words = |ws: &[cst::Word], out: &mut Vec<String>| {
        for w in ws {
            collect_word(w, out);
        }
    };
    match cmd {
        Cmd::Simple(s) => {
            words(&s.words, out);
            collect_redir_words(&s.redirs, out);
        }
        Cmd::Subshell { body, redirs } | Cmd::BraceGroup { body, redirs } => {
            collect_script_words(body, out);
            collect_redir_words(redirs, out);
        }
        Cmd::For {
            items,
            body,
            redirs,
            ..
        } => {
            words(items, out);
            collect_script_words(body, out);
            collect_redir_words(redirs, out);
        }
        Cmd::While { cond, body, redirs } | Cmd::Until { cond, body, redirs } => {
            collect_script_words(cond, out);
            collect_script_words(body, out);
            collect_redir_words(redirs, out);
        }
        Cmd::If {
            branches,
            else_body,
            redirs,
        } => {
            collect_redir_words(redirs, out);
            for branch in branches {
                collect_script_words(&branch.cond, out);
                collect_script_words(&branch.body, out);
            }
            if let Some(body) = else_body {
                collect_script_words(body, out);
            }
        }
        Cmd::DoubleBracket { words: ws, redirs } => {
            words(ws, out);
            collect_redir_words(redirs, out);
        }
        Cmd::Case {
            subject,
            arms,
            redirs,
        } => {
            collect_word(subject, out);
            for arm in arms {
                collect_script_words(&arm.body, out);
            }
            collect_redir_words(redirs, out);
        }
        Cmd::FunctionDef { body, .. } => collect_script_words(body, out),
    }
}

#[cfg(test)]
mod tests;
