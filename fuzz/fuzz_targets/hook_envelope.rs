#![no_main]

//! HOOK-ENVELOPE target: the harness-facing I/O layer, which `is_safe_command` never reaches.
//!
//! `targets/*` sits at roughly zero coverage under the `parse` target because nothing in the
//! classifier calls it (AGENTS.md §Fuzzing). It is also the layer where a mistake fails OPEN: this
//! code runs as a PreToolUse hook, so a panic aborts the hook and the harness lets the command
//! through. Two defects were found here by hand in the 2026-07-29 review — a blank command drawing
//! an authoritative `allow`, and a wrong-typed settings key panicking — and both are exactly what a
//! target keeps catching as harness schemas drift.
//!
//! Two contracts, and the second is the one worth having:
//!
//! 1. `parse_input` never panics on arbitrary bytes. Malformed JSON must be an `Err`, not a crash.
//! 2. A target never emits an ALLOW for a command that did not classify as safe. Panic-freedom
//!    alone would miss the blank-command bug, which never panicked — it answered `allow`.

use libfuzzer_sys::fuzz_target;

/// Decision tokens that GRANT. Kept as strings because each harness spells its own: Claude nests
/// `permissionDecision`, Copilot has it flat, Cursor says `permission`, Grok says `decision`.
const GRANTS: &[&str] = &["\"allow\"", "\"approve\"", "\"accept\""];

fuzz_target!(|data: &[u8]| {
    let stdin = String::from_utf8_lossy(data);

    for target in safe_chains::targets::registry() {
        let Some(format) = target.hook_format() else {
            continue;
        };

        // Contract 1: arbitrary bytes in, no panic. An Err is the correct outcome for garbage.
        let Ok(input) = format.parse_input(&stdin) else {
            continue;
        };

        // Contract 2: whatever the envelope parsed to, a grant may only follow a safe verdict.
        // A blank command is nothing to classify, so it can never justify one either — that exact
        // case shipped as an `allow` until it was found by hand.
        let verdict = safe_chains::command_verdict(&input.command);
        let response = format.render_response(verdict);
        let granted = GRANTS.iter().any(|g| response.stdout.contains(g));

        if granted {
            assert!(
                verdict.is_allowed(),
                "{} granted on a non-safe verdict: command {:?} -> {:?}, emitted `{}`",
                target.name(),
                input.command,
                verdict,
                response.stdout
            );
            assert!(
                !input.command.trim().is_empty(),
                "{} granted on a BLANK command, which classifies nothing: emitted `{}`",
                target.name(),
                response.stdout
            );
        }

        // The gated paths must not leak a grant either: whatever a target emits when it refuses or
        // escalates, it must never be a token some harness reads as approval.
        for gated in [format.render_deny("refused"), format.render_ask("confirm")] {
            for grant in GRANTS {
                assert!(
                    !gated.stdout.contains(grant),
                    "{} leaked {grant} on a gated path: `{}`",
                    target.name(),
                    gated.stdout
                );
            }
        }
    }
});
