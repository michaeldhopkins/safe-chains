//! The runtime hook: reading a harness's JSON envelope off stdin and answering in its dialect.
//!
//! The binary's half. `safe_chains::targets` knows each harness's format and what it can be told;
//! this drives one request through it — install the directory context, classify, log, and emit the
//! allow / deny / ask / context the harness understands.
//!
//! Split out of `main.rs` when the file-length gate went in: `main.rs` was over the limit, and the
//! hook is a self-contained surface that shares nothing with argument parsing but `process::exit`.

use std::io::{self, Read, Write};
use std::process;

use safe_chains::targets::{self, HookFormat};
use safe_chains::verdict::Verdict;

use crate::HOW_IT_WORKS_URL;


pub fn run_hook_for(target_name: &str, log_mode: safe_chains::decisionlog::Mode) -> ! {
    let Some(target) = targets::find(target_name) else {
        eprintln!("Unknown tool: {target_name}. Run with --list-tools to see candidates.");
        process::exit(1);
    };
    let Some(format) = target.hook_format() else {
        eprintln!(
            "{}: this target does not use a runtime hook (config-only integration).",
            target.display_name()
        );
        process::exit(1);
    };
    run_hook_format(format, target_name, log_mode);
}

/// The "outside the working directory" clause, NAMING the cwd when the harness reported one — so a
/// directory MISMATCH (the agent was launched from the wrong repo, a common and easy-to-forget
/// mistake) is visible in the message. Without naming it, the user can't tell "I meant to be
/// elsewhere" from "this command genuinely overreaches".
pub fn run_hook_format(
    format: &dyn HookFormat,
    target_name: &str,
    log_mode: safe_chains::decisionlog::Mode,
) -> ! {
    // Claude's own permission files are trust ONLY when Claude is the harness being served.
    // Every other target gets safe-chains' own classification and nothing borrowed.
    if target_name == "claude" {
        safe_chains::trust_claude_config();
    }
    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_err() {
        process::exit(0);
    }

    let Ok(input) = format.parse_input(&buf) else {
        process::exit(0);
    };


    // HP-19: install the harness cwd/root so relative paths resolve against the real
    // directory for the whole evaluation (verdict and explainer). Most harnesses send `cwd`
    // but no distinct project `root`; default root to cwd so the workspace boundary (and the
    // "reaches above your workspace" nudge) engages with the one directory we do know.
    let _ctx = safe_chains::pathctx::enter(safe_chains::pathctx::PathCtx {
        cwd: input.cwd.clone(),
        root: input.root.clone().or_else(|| input.cwd.clone()),
        session_id: input.session_id.clone(),
    });
    // The auto-approve ceiling comes from the write-protected user config (`~/.config/safe-chains.toml`,
    // `level = "…"`). Absent → the default developer band. An UPPER level (network-admin) RAISES it —
    // git push / bulk-object-read become reachable; a LOWER level (reader/editor) TIGHTENS it — a read-
    // only or no-destroy plan, gating writes the default would allow. Both funnel through the same
    // `<= threshold` gate as the CLI's `--level`. The pathctx (cwd/root) is already installed above.
    let (threshold, engine_level) = safe_chains::configured_hook_ceiling();
    let verdict = safe_chains::command_verdict_ceilinged(&input.command, threshold, engine_level);

    // The decision log (`--log` / `--log-everything`, off otherwise). Recorded at each point an
    // outcome becomes FINAL, never before: the coverage fallback below can still turn a denial into
    // an approval, so a single call up here would mislabel every command the user's own grants
    // cover. `record` checks the mode before building anything, so under `--log` the two approval
    // sites cost a comparison.
    // The level in force, in the SAME vocabulary `--explain` uses. Taking `threshold.to_string()`
    // here recorded the legacy band name (`safe-write`) for a run `--explain` called `developer`.
    let log_level = engine_level.map_or_else(
        || safe_chains::engine::bridge::default_band_top_name().to_string(),
        |l| l.name.clone(),
    );
    let log_ctx = safe_chains::decisionlog::Context {
        command: &input.command,
        cwd: input.cwd.as_deref(),
        root: input.root.as_deref(),
        session_id: input.session_id.as_deref(),
        harness: target_name,
        level: &log_level,
    };
    use safe_chains::decisionlog::{Outcome as LogOutcome, record as log_decision};

    // `respond` owns the grant rule, including that a BLANK command grants nothing: it classifies
    // as inert, but inert-about-nothing is not something to approve. Routed through the shared seam
    // so the rule cannot be bypassed by reaching for `render_response` directly.
    if let Some(response) = targets::respond(format, &input.command, verdict) {
        log_decision(log_mode, LogOutcome::Allowed, &log_ctx, None);
        let _ = io::stdout().write_all(response.stdout.as_bytes());
        process::exit(response.exit_code);
    }
    if verdict.is_allowed() {
        // Allowed but not GRANTABLE (a blank command): safe-chains says nothing and the harness
        // decides. Recorded as an abstain rather than an approval, because that is what it is —
        // `may_grant` exists precisely because approving nothing is not an approval.
        log_decision(log_mode, LogOutcome::Abstained, &log_ctx, None);
        process::exit(0);
    }

    // Coverage fallback: the built-in/pattern classifier (also honoring the user's own
    // `~/.claude/settings.json` `permissions.allow` grants), computed UNDER the configured engine level
    // so a covered command respects that level's rule (an `editor` plan's forbidden worktree destroy
    // classifies denied here too). Its REAL level in `overall` is then held under the SAME `<=
    // threshold` ceiling — so a lower `level` (reader) can't have the write it just gated re-admitted.
    // At the default band both are no-ops (no engine level, coverage `<= SafeWrite`).
    let explanation = safe_chains::explain_with_coverage_at_level(&input.command, engine_level);

    if let Verdict::Allowed(level) = explanation.overall
        && level <= threshold
    {
        log_decision(log_mode, LogOutcome::Allowed, &log_ctx, Some(&explanation));
        let response = format.render_response(Verdict::Allowed(level));
        let _ = io::stdout().write_all(response.stdout.as_bytes());
        process::exit(response.exit_code);
    }

    // Past this point every exit is a non-approval — Deny, Ask, a surfaced explanation, the
    // overreach nudge, or a silent fall-through to the harness's own prompt. They differ in what
    // the HARNESS is told, not in what safe-chains decided, so the log records once here rather
    // than at each of the five. A parse failure is kept distinct: it is a correct refusal, but a
    // RISE in them is how a parser regression shows up in the field, and nothing else surfaces it.
    let outcome =
        if explanation.parsed { LogOutcome::Denied } else { LogOutcome::Unparseable };
    log_decision(log_mode, outcome, &log_ctx, Some(&explanation));

    // GATED command. What the hook emits depends on the harness's capabilities
    // (docs/design/harness-capability-model.md):
    //  - Deny (e.g. Codex): no interactive approval, so VETO it (silence would just run it — its
    //    sandbox even permits broad reads). Escape valve is a config-level exception.
    //  - Ask  (e.g. Antigravity): escalate to an in-the-moment human prompt.
    //  - Defer (e.g. Claude): fall through to context/nudge/silent so the harness's own prompt decides.
    // When the command was gated because it reaches OUTSIDE the workspace, fold that specific reason
    // into the Deny/Ask message so the human/model sees *why* — Defer surfaces it via render_context
    // below, but Deny/Ask exit here, so without this they'd get only the generic reason.
    let overreach = safe_chains::workspace_overreach(&input.command);
    let overreach_why = overreach.as_ref().map(|(path, reason)| reason.message(path));
    match format.gated_policy() {
        safe_chains::targets::GatedPolicy::Deny => {
            // The copy follows what we EMIT, not which harness this is: here we emit deny and the
            // harness honours it, so "the command did not run" is accurate. On the Ask arm below
            // the same refusal must not say that. See docs/design/refusal-copy.md rule 1.
            // No `HOW_IT_WORKS_URL` appended. The builder already closes with the issues link for an
            // unknown command, and two trailing URLs read as boilerplate — the reader skips both.
            // The reach cause carries its own remedy in `ReachReason::message`.
            let reason = safe_chains::refusal::Refusal {
                    outcome: safe_chains::refusal::Outcome::DidNotRun,
                    cause: match &overreach_why {
                        Some(why) => safe_chains::refusal::Cause::Reach(why.clone()),
                        None => safe_chains::refusal::Cause::no_entry(&input.command),
                    },
                }
            .render();
            let response = format.render_deny(&reason);
            let _ = io::stdout().write_all(response.stdout.as_bytes());
            process::exit(response.exit_code);
        }
        safe_chains::targets::GatedPolicy::Ask => {
            // Same refusal, different EMISSION: the harness will run its own approval flow, so
            // this must not claim the command was stopped.
            // No `HOW_IT_WORKS_URL` appended. The builder already closes with the issues link for an
            // unknown command, and two trailing URLs read as boilerplate — the reader skips both.
            // The reach cause carries its own remedy in `ReachReason::message`.
            let reason = safe_chains::refusal::Refusal {
                    outcome: safe_chains::refusal::Outcome::GoesToHuman,
                    cause: match &overreach_why {
                        Some(why) => safe_chains::refusal::Cause::Reach(why.clone()),
                        None => safe_chains::refusal::Cause::no_entry(&input.command),
                    },
                }
            .render();
            let response = format.render_ask(&reason);
            let _ = io::stdout().write_all(response.stdout.as_bytes());
            process::exit(response.exit_code);
        }
        safe_chains::targets::GatedPolicy::Defer => {}
    }

    if explanation.should_surface() {
        let response = format.render_context(&explanation.render());
        let _ = io::stdout().write_all(response.stdout.as_bytes());
        process::exit(response.exit_code);
    }

    // The retreat's nudge: if the command wasn't auto-approved because it reaches outside the
    // workspace, say so (and how to allow it) instead of a silent prompt. Degrades to a plain
    // prompt on harnesses without additionalContext.
    if let Some((path, reason)) = overreach {
        let nudge = format!(
            "safe-chains did not auto-approve this: {}. {HOW_IT_WORKS_URL}",
            reason.message(&path)
        );
        let response = format.render_context(&nudge);
        let _ = io::stdout().write_all(response.stdout.as_bytes());
        process::exit(response.exit_code);
    }

    process::exit(0);
}

