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

/// More entries than this across the walk, or more matches in all, and the word stays unknown.
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
    let mut seen = 0usize;
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
            for entry in entries {
                seen += 1;
                if seen > MAX_ENTRIES {
                    return None;
                }
                names.push(entry.ok()?.file_name().into_string().ok()?);
            }
            if component.starts_with('.') {
                names.extend([".".to_string(), "..".to_string()]);
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
pub(crate) fn matches(pattern: &[u8], name: &[u8]) -> Option<bool> {
    if !brackets_valid(pattern) {
        return None;
    }
    let (mut pi, mut ni, mut star, mut mark) = (0, 0, None, 0);
    while ni < name.len() {
        if pi < pattern.len() && pattern[pi] == b'*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if let Some((true, width)) = step(&pattern[pi..], name[ni]) {
            pi += width;
            ni += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ni = mark;
        } else {
            return Some(false);
        }
    }
    Some(pattern[pi..].iter().all(|&b| b == b'*'))
}

fn brackets_valid(mut pattern: &[u8]) -> bool {
    while let Some((&b, rest)) = pattern.split_first() {
        if b == b'[' {
            let Some((_, _, width)) = bracket_set(rest) else {
                return false;
            };
            pattern = &pattern[width..];
        } else {
            pattern = rest;
        }
    }
    true
}

fn bracket_set(rest: &[u8]) -> Option<(bool, &[u8], usize)> {
    let (negate, body) = match rest.split_first() {
        Some((b'!' | b'^', tail)) => (true, tail),
        _ => (false, rest),
    };
    let end = body.iter().skip(1).position(|&b| b == b']')? + 1;
    let set = &body[..end];
    (!set.contains(&b'[')).then_some((negate, set, 2 + usize::from(negate) + end))
}

fn step(pattern: &[u8], c: u8) -> Option<(bool, usize)> {
    let (&p, rest) = pattern.split_first()?;
    match p {
        b'?' => Some((true, 1)),
        b'[' => {
            let (negate, set, width) = bracket_set(rest)?;
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
            Some((hit != negate, width))
        }
        _ => Some((p == c, 1)),
    }
}

#[cfg(test)]
mod tests {
    use super::{expand, matches};
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
            ("[!]", "a", None),
            ("[^]", "a", None),
            ("[!]a]", "b", Some(true)),
            ("[!]a]", "]", Some(false)),
            ("[^]a]", "a", Some(false)),
            ("[[:alpha:]]", "a", None),
            ("[ab", "a", None),
            ("a[b", "x", None),
            ("*", "", Some(true)),
            ("[*]", "*", Some(true)),
            ("[*]", "a", Some(false)),
            ("******.", "systemd-networkd.service", Some(false)),
            ("*a*a*a*a*a*a*a*a*a*a*b", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", Some(false)),
            ("a*b*c*d*e*f*g*h*i*j", "abcdefghij", Some(true)),
        ];
        for (p, n, want) in cases {
            assert_eq!(matches(p.as_bytes(), n.as_bytes()), *want, "{p} vs {n}");
        }
    }

    #[test]
    fn a_dot_pattern_matches_the_folder_and_its_parent() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".x"), "").expect("write");
        let at = dir.path().canonicalize().expect("canonical").to_string_lossy().into_owned();
        let _cwd = crate::pathctx::enter_cwd(Some(at));
        assert_eq!(expand(&crate::cst::Word::literal(".*")), Some(vec![".".into(), "..".into(), ".x".into()]));
        assert_eq!(expand(&crate::cst::Word::literal("*")), Some(vec!["*".into()]));
    }

    #[test]
    fn expand_counts_entries_across_nested_directories() {
        let dir = tempfile::tempdir().expect("tempdir");
        for d in 0..5 {
            let sub = dir.path().join(format!("d{d}"));
            std::fs::create_dir_all(&sub).expect("mkdir");
            for f in 0..900 {
                std::fs::write(sub.join(format!("f{f}")), "").expect("write");
            }
        }
        let at = dir.path().canonicalize().expect("canonical").to_string_lossy().into_owned();
        let _cwd = crate::pathctx::enter_cwd(Some(at));
        assert_eq!(expand(&crate::cst::Word::literal("d*/none*")), None);
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

        #[test]
        fn repeating_stars_never_changes_the_match(
            parts in prop::collection::vec("[a-d?]|\\[[!^]?\\]?[a-d]-?[a-d]\\]", 1..12),
            runs in prop::collection::vec(1usize..8, 1..12),
            lead in 0usize..6,
            trail in 0usize..6,
            name in "[a-e\\]]{0,40}",
        ) {
            let mut single = "*".repeat(usize::from(lead > 0));
            let mut repeated = "*".repeat(lead);
            for (i, part) in parts.iter().enumerate() {
                if i > 0 {
                    single.push('*');
                    repeated.push_str(&"*".repeat(runs[i % runs.len()]));
                }
                single.push_str(part);
                repeated.push_str(part);
            }
            if trail > 0 {
                single.push('*');
                repeated.push_str(&"*".repeat(trail));
            }
            let got = matches(repeated.as_bytes(), name.as_bytes());
            prop_assert!(got.is_some(), "{}", repeated);
            prop_assert_eq!(got, matches(single.as_bytes(), name.as_bytes()), "{} vs {} on {}", repeated, single, name);
        }
    }
}
