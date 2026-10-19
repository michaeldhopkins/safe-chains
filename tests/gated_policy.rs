//! What each target does with a gated command, by its kind and the harness's permission mode.
//!
//! Only Grok splits: a not-yet-approved command goes to its permission prompt in a mode where one
//! is shown, and everything else is refused as before. Every other target must answer exactly what
//! its `gated_policy` answered before the split existed, whatever the kind and the mode.

#[path = "support/formats.rs"]
mod formats;
use formats::every_hook_format;
use proptest::prelude::*;
use safe_chains::explain_fix::gate::GateKind;

const SPLIT_TARGETS: &[&str] = &["grok"];

proptest! {
    #[test]
    fn every_other_target_is_unchanged_for_both_kinds(mode in proptest::option::of(".{0,24}"), not_yet in any::<bool>()) {
        let kind = if not_yet { GateKind::NotYet } else { GateKind::Boundary };
        for (name, format) in every_hook_format() {
            if SPLIT_TARGETS.contains(&name) {
                continue;
            }
            prop_assert_eq!(format.gated_policy_for(kind, mode.as_deref()), format.gated_policy(), "{}", name);
        }
    }

}

#[test]
fn the_split_list_names_real_targets() {
    let names: Vec<&str> = every_hook_format().into_iter().map(|(n, _)| n).collect();
    for split in SPLIT_TARGETS {
        assert!(names.contains(split), "{split} is not a hook target");
    }
}
