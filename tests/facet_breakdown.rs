//! `facet_breakdown`, the "resolved profile" part of `--explain`.

use safe_chains::facet_breakdown;

/// `--explain`'s facet breakdown describes ONE command. A chain gets the note pointing at the
/// per-segment view instead of a profile, because the flat word split would read the second
/// command's words as the first's flags and invent a profile neither segment has.
#[test]
fn facet_breakdown_profiles_a_single_command_and_declines_a_chain() {
    const ONE_COMMAND_NOTE: &str = "facet breakdown covers one command at a time";
    let single = facet_breakdown("rm -rf /");
    assert!(single.contains("resolved profile:"), "a single command gets its profile: {single}");
    assert!(!single.contains(ONE_COMMAND_NOTE), "{single}");

    for chain in ["ls && rm -rf /", "ls || rm -rf /", "ls; rm -rf /"] {
        let out = facet_breakdown(chain);
        assert!(out.contains(ONE_COMMAND_NOTE), "{chain}: {out}");
        assert!(!out.contains("resolved profile:"), "{chain}: {out}");
    }
}
