//! Expanding a glob against the working directory at decision time.
//!
//! A glob that starts with a pattern character can match a file named `-delete`, so without the
//! folder it is refused as any unknown word is. With the folder known, the matches are read: when
//! none begins with `-` or holds a newline, they are the words the command receives, and it is
//! classified on them.
//!
//! The listing is taken when the hook decides, not when the shell runs the command, so a file
//! created in between is not seen. That window is an accepted risk, recorded in
//! `docs/design/hard-problems.md` (HP-5). So are shell options that change what a glob matches (`dotglob`,
//! `nocaseglob`, `GLOBIGNORE`, `nullglob`) when the session set them before this command: bash's
//! defaults are assumed.

use crate::cst::{Word, WordPart};

/// More entries than this in one directory, or more matches in all, and the word stays unknown.
const MAX_ENTRIES: usize = 4096;
const MAX_MATCHES: usize = 256;

/// The words bash makes of `word` in the current directory: `Some(matches)` (sorted, as bash sorts
/// them; the pattern itself when nothing matches), or `None` when the word is not a plain glob or
/// its matches cannot be listed.
pub(super) fn expand(word: &Word) -> Option<Vec<String>> {
    let [WordPart::Lit(pattern)] = word.0.as_slice() else {
        return None;
    };
    if !pattern.contains(['*', '?', '[']) || pattern.contains(['$', '{', '\\', '~']) || pattern.starts_with('/') {
        return None;
    }
    let cwd = crate::pathctx::cwd().filter(|c| c.starts_with('/') && c != crate::pathctx::UNRESOLVED_CWD)?;
    let mut found = vec![String::new()];
    for component in pattern.split('/') {
        let mut next = Vec::new();
        for prefix in &found {
            if !component.contains(['*', '?', '[']) {
                next.push(join(prefix, component));
                continue;
            }
            let dir = if prefix.is_empty() { cwd.clone() } else { format!("{cwd}/{prefix}") };
            // The folder itself must list: when it cannot (a `cd` into a folder that does not
            // exist fails, and the glob then expands wherever the shell still is), nothing is known.
            let entries = match std::fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(_) if prefix.is_empty() => return None,
                Err(_) => continue,
            };
            let mut names = Vec::new();
            for entry in entries.take(MAX_ENTRIES + 1) {
                names.push(entry.ok()?.file_name().into_string().ok()?);
            }
            if names.len() > MAX_ENTRIES {
                return None;
            }
            for name in names {
                if (!name.starts_with('.') || component.starts_with('.')) && matches(component.as_bytes(), name.as_bytes())? {
                    next.push(join(prefix, &name));
                }
            }
        }
        found = next;
        if found.len() > MAX_MATCHES {
            return None;
        }
    }
    let mut found: Vec<String> = found.into_iter().filter(|m| std::fs::symlink_metadata(format!("{cwd}/{m}")).is_ok()).collect();
    found.sort();
    Some(if found.is_empty() { vec![pattern.clone()] } else { found })
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() { name.to_string() } else { format!("{prefix}/{name}") }
}

/// fnmatch for `*`, `?` and `[…]` (with `!`/`^` negation and ranges), byte-wise. `None` for a
/// bracket form not modeled (`[[:alpha:]]`, an unclosed `[`), so the caller gives up.
fn matches(pattern: &[u8], name: &[u8]) -> Option<bool> {
    if pattern.iter().filter(|&&b| b == b'*').count() > 8 {
        return None;
    }
    let Some((&p, rest)) = pattern.split_first() else {
        return Some(name.is_empty());
    };
    match p {
        b'*' => {
            for skip in 0..=name.len() {
                if matches(rest, &name[skip..])? {
                    return Some(true);
                }
            }
            Some(false)
        }
        b'?' => Some(!name.is_empty() && matches(rest, &name[1..])?),
        b'[' => {
            let close = rest.iter().skip(1).position(|&b| b == b']')? + 1;
            let set = &rest[..close];
            if set.contains(&b'[') {
                return None;
            }
            let (negate, set) = match set.first() {
                Some(b'!' | b'^') => (true, &set[1..]),
                _ => (false, set),
            };
            let Some((&c, tail)) = name.split_first() else { return Some(false) };
            let mut hit = false;
            let mut i = 0;
            while i < set.len() {
                if i + 2 < set.len() && set[i + 1] == b'-' {
                    hit |= (set[i]..=set[i + 2]).contains(&c);
                    i += 3;
                } else {
                    hit |= set[i] == c;
                    i += 1;
                }
            }
            Some(hit != negate && matches(&rest[close + 1..], tail)?)
        }
        _ => Some(name.first() == Some(&p) && matches(rest, &name[1..])?),
    }
}

#[cfg(test)]
mod tests {
    use super::matches;
    use proptest::prelude::*;

    #[test]
    fn matches_as_fnmatch_does() {
        let cases: &[(&str, &str, Option<bool>)] = &[
            ("*.txt", "a.txt", Some(true)),
            ("*.txt", "a.txt.bak", Some(false)),
            ("?", "-", Some(true)),
            ("[ab]*", "b-x", Some(true)),
            ("[!a]*", "a", Some(false)),
            ("[a-c]", "b", Some(true)),
            ("[]]", "]", Some(true)),
            ("[[:alpha:]]", "a", None),
            ("[ab", "a", None),
            ("*", "", Some(true)),
        ];
        for (p, n, want) in cases {
            assert_eq!(matches(p.as_bytes(), n.as_bytes()), *want, "{p} vs {n}");
        }
    }

    proptest! {
        /// A literal pattern matches only itself, and `*` matches anything.
        #[test]
        fn literals_match_themselves(name in "[a-z.-]{0,12}", other in "[a-z.-]{0,12}") {
            prop_assert_eq!(matches(name.as_bytes(), other.as_bytes()), Some(name == other));
            prop_assert_eq!(matches(b"*", name.as_bytes()), Some(true));
            let prefixed = format!("{name}*");
            prop_assert_eq!(matches(prefixed.as_bytes(), format!("{name}{other}").as_bytes()), Some(true));
        }
    }
}
