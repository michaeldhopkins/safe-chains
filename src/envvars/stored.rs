//! The names a tool may not store for later shells (`mise set NAME=VALUE`, which `mise activate`
//! exports into every later shell below the file): `envvars.toml`'s `[stored]` list. A stored value
//! changes what every later command runs or trusts, so a listed name is refused whatever the value;
//! any other is judged as `export NAME=VALUE` is.

use std::sync::LazyLock;

use serde::Deserialize;

use crate::verdict::Verdict;

#[derive(Deserialize)]
struct File {
    stored: Stored,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    #[serde(rename = "because")]
    _because: String,
    names: Vec<String>,
}

fn parsed() -> Stored {
    let file: File = toml::from_str(include_str!("../../envvars.toml")).expect("embedded envvars.toml must parse");
    file.stored
}

/// Each name in lowercase, as a prefix and, for a name with a `*`, the suffix after it.
static PATTERNS: LazyLock<Vec<(String, Option<String>)>> = LazyLock::new(|| {
    parsed()
        .names
        .iter()
        .map(|name| {
            let name = name.to_ascii_lowercase();
            match name.split_once('*') {
                Some((pre, suf)) => {
                    assert!(!suf.contains('*'), "envvars.toml: stored `{name}` has more than one `*`");
                    (pre.to_string(), Some(suf.to_string()))
                }
                None => (name, None),
            }
        })
        .collect()
});

/// The verdict for an assignment a tool stores for later shells: refused for a name in `[stored]`,
/// whatever the value, and otherwise judged as a lasting assignment.
pub(crate) fn stored_assignment_verdict(name: &str, value: &str) -> Verdict {
    if changes_later_shells(name) {
        return Verdict::Denied;
    }
    super::statement_assignment_verdict(name, value, true)
}

fn changes_later_shells(name: &str) -> bool {
    let name = name.strip_suffix('+').unwrap_or(name).to_ascii_lowercase();
    PATTERNS.iter().any(|(pre, suf)| match suf {
        None => name == *pre,
        Some(suf) => name.len() >= pre.len() + suf.len() && name.starts_with(pre.as_str()) && name.ends_with(suf.as_str()),
    })
}

#[cfg(test)]
#[path = "stored_tests.rs"]
mod tests;
