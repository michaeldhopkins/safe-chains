#![no_main]

//! LEVEL-SOUNDNESS target: tightening the configured level can never LOOSEN a verdict.
//!
//! The level is the one knob a user turns to be safer — `level = "reader"` in the write-protected
//! config is a promise that less is approved than at `developer`, not merely different. If any
//! command is refused at a looser level and approved at a stricter one, that promise is broken and
//! the knob is worse than useless, because it reads as a tightening while opening something.
//!
//! `engine/testgen.rs` already proptests the level ALGEBRA over synthesized capabilities. This is
//! the other half: real command strings through `command_verdict_ceilinged`, the seam the CLI's
//! `--level` and the hook's configured level both funnel through, so it covers the projection and
//! the coverage fallback rather than the algebra alone.
//!
//! The order is a partial one, and asserting a total order would be wrong: `local-admin` and
//! `network-admin` are SIBLINGS that flex disjoint facets (local privilege versus remote reach), so
//! neither admits the other. Each chain below is a path through that lattice, and the property is
//! asserted along chains only.

use libfuzzer_sys::fuzz_target;

/// Paths through the level lattice, strictest first. Every adjacent pair must be non-loosening.
const CHAINS: &[&[&str]] = &[
    &["paranoid", "reader", "editor", "developer", "local-admin", "yolo"],
    &["paranoid", "reader", "editor", "developer", "network-admin", "yolo"],
];

fuzz_target!(|data: &[u8]| {
    let command = String::from_utf8_lossy(data);

    for chain in CHAINS {
        let mut previous: Option<(&str, bool)> = None;
        for name in *chain {
            let Some((threshold, engine_level)) = safe_chains::level_ceiling(name) else {
                continue; // not a level this build knows; nothing to assert
            };
            let allowed =
                safe_chains::command_verdict_ceilinged(&command, threshold, engine_level)
                    .is_allowed();

            if let Some((stricter_name, stricter_allowed)) = previous {
                // The implication that matters: allowed at the STRICTER level implies allowed at
                // the looser one. The converse is fine and expected — that is what tightening does.
                assert!(
                    !stricter_allowed || allowed,
                    "tightening LOOSENED a verdict: `{command}` is approved at the stricter \
                     `{stricter_name}` but refused at the looser `{name}`"
                );
            }
            previous = Some((name, allowed));
        }
    }
});
