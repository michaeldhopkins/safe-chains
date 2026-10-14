//! What "researched to the reference target" means for one `[[command]]` entry, as pure functions
//! over its parsed TOML. Each criterion is a structural check of an artefact the research procedure
//! in AGENTS.md produces; none of them can judge whether the research is RIGHT, which is what review
//! is for. They make a skipped step visible and countable.

use std::collections::BTreeSet;

use toml::Value;

pub const ENTRY: &str = "entry";
pub const DESCRIPTION: &str = "description";
pub const VERSION: &str = "version";
pub const EXAMPLES: &str = "examples";
pub const PROFILED: &str = "profiled";
pub const NO_TOLERANCE: &str = "no-tolerance";

pub const ALL: [&str; 6] = [ENTRY, DESCRIPTION, VERSION, EXAMPLES, PROFILED, NO_TOLERANCE];

/// Below this a description cannot cover the axes AGENTS.md asks for; the shortest complete ones
/// in the registry (`basename`, `true`) run to about 200 characters.
pub const MIN_DESCRIPTION: usize = 160;

const NETWORK_WORDS: [&str; 11] =
    ["network", "remote", "offline", "internet", "http", "server", "api", "online", "download", "upload", "fetch"];
const VELOCITY_WORDS: [&str; 10] =
    ["release", "stable", "unchanged", "version", "maintain", "cadence", "active", "deprecated", "archived", "frozen"];
const INTERNAL_WORDS: [&str; 7] = ["saferead", "safewrite", "inert", "allowlist", "allow-list", "safe-chains", "safe chains"];

/// Whether a word of the text starts with `stem`, so "releases" counts for "release" but
/// "interactive" does not count for "active", nor "rapid" for "api".
fn has_word(lower: &str, stem: &str) -> bool {
    lower.split(|c: char| !c.is_alphanumeric()).any(|w| w.starts_with(stem))
}

fn text<'a>(table: &'a Value, key: &str) -> Option<&'a str> {
    table.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}

fn flag(table: &Value, key: &str) -> bool {
    table.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn list_len(table: &Value, key: &str) -> usize {
    table.get(key).and_then(Value::as_array).map_or(0, Vec::len)
}

/// Every table of the entry: the command itself and every `sub`, however deep.
fn nodes(table: &Value) -> Vec<&Value> {
    let mut out = vec![table];
    for sub in table.get("sub").and_then(Value::as_array).into_iter().flatten() {
        out.extend(nodes(sub));
    }
    out
}

/// A safety profile, not a summary: long enough to cover the axes, says where the network stands
/// and how fast upstream moves, and describes the command rather than how it is classified here.
pub fn description_ok(entry: &Value) -> bool {
    let Some(d) = text(entry, "description") else { return false };
    let lower = d.to_lowercase();
    d.chars().count() >= MIN_DESCRIPTION
        && NETWORK_WORDS.iter().any(|w| has_word(&lower, w))
        && VELOCITY_WORDS.iter().any(|w| has_word(&lower, w))
        && !INTERNAL_WORDS.iter().any(|w| lower.contains(w))
}

pub fn version_ok(entry: &Value) -> bool {
    text(entry, "researched_version").is_some_and(|v| !v.eq_ignore_ascii_case("unknown"))
}

/// One invocation that must be approved and one that must not, so the research is pinned by a
/// test. A command that is wholly above the line has nothing to approve, so it owes only a denial.
pub fn examples_ok(entry: &Value) -> bool {
    let denied = list_len(entry, "examples_denied") > 0;
    if flag(entry, "candidate") { denied } else { denied && list_len(entry, "examples_safe") > 0 }
}

/// `profile` is the facet-derived successor to `candidate = true`: every node still marked a
/// candidate without an archetype is a decision recorded in the old vocabulary.
pub fn profiled_ok(entry: &Value) -> bool {
    nodes(entry).into_iter().all(|n| !flag(n, "candidate") || text(n, "profile").is_some())
}

/// Blanket flag tolerance admits flags nobody researched (TODO.md, "Retire blanket flag tolerance").
pub fn no_tolerance_ok(entry: &Value) -> bool {
    nodes(entry)
        .into_iter()
        .all(|n| !flag(n, "tolerate_unknown_short") && !flag(n, "tolerate_unknown_long"))
}

type Check = (&'static str, fn(&Value) -> bool);

/// The criteria one `[[command]]` table misses.
pub fn missing(entry: &Value) -> BTreeSet<&'static str> {
    let checks: [Check; 5] = [
        (DESCRIPTION, description_ok),
        (VERSION, version_ok),
        (EXAMPLES, examples_ok),
        (PROFILED, profiled_ok),
        (NO_TOLERANCE, no_tolerance_ok),
    ];
    checks.iter().filter(|(_, ok)| !ok(entry)).map(|(name, _)| *name).collect()
}

/// Whether the entry routes through custom Rust code anywhere in its tree.
pub fn custom(entry: &Value) -> bool {
    nodes(entry).into_iter().any(|n| text(n, "handler").is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD_DESCRIPTION: &str = "Prints the lines of local files that match a pattern. Reads only the files it is \
        given; it never writes, and it never touches the network. Part of a POSIX toolset whose flag surface has \
        been stable for decades, with releases a few times a year.";

    fn entry(body: &str) -> Value {
        let text = format!("name = \"x\"\nurl = \"https://example.com\"\n{body}");
        toml::from_str(&text).expect("fixture parses")
    }

    fn complete() -> String {
        format!(
            "description = \"{GOOD_DESCRIPTION}\"\nresearched_version = \"1.2\"\nexamples_safe = [\"x a\"]\nexamples_denied = [\"x --rm\"]\n"
        )
    }

    #[test]
    fn a_complete_entry_misses_nothing() {
        assert!(missing(&entry(&complete())).is_empty());
    }

    #[test]
    fn an_empty_entry_misses_every_entry_level_criterion() {
        let m = missing(&entry(""));
        assert_eq!(m, [DESCRIPTION, VERSION, EXAMPLES].into_iter().collect());
    }

    #[test]
    fn a_short_description_is_not_a_profile() {
        let e = entry(&complete().replace(GOOD_DESCRIPTION, "Prints matching lines; no network; stable release."));
        assert!(!description_ok(&e));
    }

    #[test]
    fn a_description_must_place_the_network_and_the_release_pace() {
        let no_network = GOOD_DESCRIPTION.replace("touches the network", "touches anything else");
        assert!(!description_ok(&entry(&complete().replace(GOOD_DESCRIPTION, &no_network))));
        let no_pace = GOOD_DESCRIPTION.replace("has been stable for decades, with releases", "has stayed put, with tags");
        assert!(!description_ok(&entry(&complete().replace(GOOD_DESCRIPTION, &no_pace))));
    }

    #[test]
    fn a_description_describes_the_command_not_its_classification() {
        for word in ["Classified SafeRead.", "It is inert.", "On the Allowlist.", "Safe Chains approves it."] {
            let internal = format!("{GOOD_DESCRIPTION} {word}");
            assert!(!description_ok(&entry(&complete().replace(GOOD_DESCRIPTION, &internal))), "{word}");
        }
    }

    #[test]
    fn a_word_inside_another_word_does_not_count() {
        let pace = GOOD_DESCRIPTION.replace("has been stable for decades, with releases", "is interactive and unstable, with tags");
        assert!(!description_ok(&entry(&complete().replace(GOOD_DESCRIPTION, &pace))));
        let network = GOOD_DESCRIPTION.replace("touches the network", "is a rapid observer");
        assert!(!description_ok(&entry(&complete().replace(GOOD_DESCRIPTION, &network))));
    }

    #[test]
    fn an_unknown_version_is_no_version() {
        assert!(!version_ok(&entry("researched_version = \"Unknown\"")));
        assert!(!version_ok(&entry("researched_version = \"  \"")));
        assert!(version_ok(&entry("researched_version = \"git 2.51\"")));
    }

    #[test]
    fn a_candidate_command_owes_only_a_denial() {
        assert!(examples_ok(&entry("candidate = true\nexamples_denied = [\"x\"]")));
        assert!(!examples_ok(&entry("candidate = true")));
        assert!(!examples_ok(&entry("examples_denied = [\"x\"]")));
        assert!(!examples_ok(&entry("examples_safe = [\"x\"]")));
    }

    #[test]
    fn a_candidate_anywhere_in_the_tree_needs_a_profile() {
        let deep = "[[sub]]\nname = \"a\"\n[[sub.sub]]\nname = \"b\"\ncandidate = true\n";
        assert!(!profiled_ok(&entry(deep)));
        let profiled = format!("{deep}profile = \"remote-mutate\"\n");
        assert!(profiled_ok(&entry(&profiled)));
        assert!(!profiled_ok(&entry("candidate = true")));
    }

    #[test]
    fn tolerance_anywhere_in_the_tree_is_owed() {
        assert!(!no_tolerance_ok(&entry("[[sub]]\nname = \"a\"\ntolerate_unknown_long = true\n")));
        assert!(!no_tolerance_ok(&entry("tolerate_unknown_short = true")));
        assert!(no_tolerance_ok(&entry("tolerate_unknown_short = false")));
    }

    #[test]
    fn custom_code_is_found_at_any_depth() {
        assert!(custom(&entry("[[sub]]\nname = \"a\"\nhandler = \"h\"\n")));
        assert!(!custom(&entry("")));
    }
}
