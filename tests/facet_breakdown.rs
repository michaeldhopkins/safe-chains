//! `facet_breakdown`, the "resolved profile" part of `--explain`.

use safe_chains::facet_breakdown;

/// `--explain`'s facet breakdown describes ONE command. A chain gets the note pointing at the
/// per-segment view instead of a profile, because the flat word split would read the second
/// command's words as the first's flags and invent a profile neither segment has.
#[test]
fn facet_breakdown_profiles_a_single_command_and_declines_a_chain() {
    const ONE_COMMAND_NOTE: &str = "facet breakdown covers one command at a time";
    let single = facet_breakdown("rm -rf /", false);
    assert!(single.contains("resolved profile:"), "a single command gets its profile: {single}");
    assert!(!single.contains(ONE_COMMAND_NOTE), "{single}");

    for chain in ["ls && rm -rf /", "ls || rm -rf /", "ls; rm -rf /"] {
        let out = facet_breakdown(chain, false);
        assert!(out.contains(ONE_COMMAND_NOTE), "{chain}: {out}");
        assert!(!out.contains("resolved profile:"), "{chain}: {out}");
    }
}

#[test]
fn the_breakdown_speaks_plainly_where_the_wording_is_approved() {
    let out = facet_breakdown("rm -rf ./build", false);
    assert!(out.contains("does                         deletes files in your project"), "{out}");
    assert!(!out.contains("operation  "), "{out}");
    assert!(!out.contains("locus.local"), "{out}");
}

#[test]
fn the_breakdown_says_how_hard_a_delete_is_to_undo() {
    let out = facet_breakdown("rm -rf ./build", false);
    assert!(out.contains("reversibility                hard to undo; it may take a backup, a rebuild or a reverse operation"), "{out}");
}

#[test]
fn the_breakdown_names_a_place_without_a_rule() {
    let out = facet_breakdown("rm -rf ~/notes", false);
    assert!(out.contains("does                         deletes files in a place safe-chains has no rule for"), "{out}");
    assert!(!out.contains("locus.local"), "{out}");
}

#[test]
fn a_refusal_in_the_breakdown_reads_in_plain_words() {
    let out = facet_breakdown("git push", false);
    assert!(out.contains("talks to another system (allowed: one of: reads or looks / creates / changes)"), "{out}");
}

#[test]
fn a_read_of_a_secret_cannot_be_taken_back_and_is_refused_for_the_secret() {
    for command in ["sops -d secrets.yaml", "cat ~/.ssh/id_rsa", "gcloud auth print-access-token"] {
        let out = facet_breakdown(command, false);
        assert!(out.contains("reversibility                cannot be taken back once seen"), "{command}: {out}");
        assert!(!out.contains("    cannot be taken back once seen (allowed"), "{command}: {out}");
    }
    let moved = facet_breakdown("mv ~/.ssh/id_rsa /tmp/x", false);
    assert!(moved.contains("secret.level                 reads a password, key or token"), "{moved}");
    assert!(!moved.contains("cannot be taken back once seen"), "a move is undone by moving back: {moved}");
    let out = facet_breakdown("sops -d secrets.yaml", false);
    assert!(
        out.contains("reads a password, key or token (allowed up to: uses sign-ins you already set up, without reading them)"),
        "{out}"
    );
}

#[test]
fn a_named_place_drops_the_words_for_a_place_with_no_rule() {
    let unnamed = facet_breakdown("cat ~/.ssh/id_rsa", false);
    assert!(unnamed.contains("reads files in a place safe-chains has no rule for"), "{unnamed}");
    let named = facet_breakdown("cat ~/.ssh/id_rsa", true);
    assert!(named.contains("does                         reads or looks\n"), "{named}");
    assert!(!named.contains("no rule for"), "{named}");
}
