//! A path spelled with a character outside ASCII, judged as every protected name it could be.
//!
//! APFS compares names case-insensitively after Unicode case folding, so `~/.awſ` (U+017F, which
//! folds to `s`) and `~/.\u{212A}ube` (the Kelvin sign, which folds to `k`) open `~/.aws` and
//! `~/.kube` on the default macOS volume. The shields fold ASCII only. Rather than carry Unicode's
//! folding and compatibility tables, every character outside ASCII is read as a wildcard for any
//! run of characters within its component, and the path is refused when a protected node could
//! then name it. A non-ASCII name that could not be a protected one is unaffected.

use super::metadata::glob_may_match;
use super::{Matcher, REGIONS, Role, home_spelling, more_restrictive};

/// The role of the most protective node `path` could name once each character outside ASCII may
/// stand for any run of characters, or `None` for an ASCII path or one no protected node fits.
pub(super) fn shield_role(path: &str) -> Option<Role> {
    if path.is_ascii() {
        return None;
    }
    let spellings: Vec<Vec<String>> = home_spelling::shield_spellings(path).iter().map(|p| pattern(p)).collect();
    REGIONS
        .nodes
        .iter()
        .filter(|n| n.applies_here() && n.fold)
        .filter(|n| spellings.iter().any(|p| could_name(&n.matcher, p)))
        .map(|n| n.role)
        .reduce(most_protective)
}

/// Of two roles, the one that reads a secret, else the one whose write then read face is higher.
fn most_protective(a: Role, b: Role) -> Role {
    if (b.reads_secret, more_restrictive(b, a)) > (a.reads_secret, false) { b } else { a }
}

/// `component` with every character outside ASCII read as `*`, or `None` when it is all ASCII.
pub(super) fn wildcard_component(component: &str) -> Option<String> {
    (!component.is_ascii()).then(|| component.chars().map(|c| if c.is_ascii() { c } else { '*' }).collect())
}

fn pattern(path: &str) -> Vec<String> {
    path.split('/').map(|c| wildcard_component(c).unwrap_or_else(|| c.to_string())).collect()
}

fn fits(pattern: &str, name: &str) -> bool {
    if pattern.contains(['*', '?', '[']) { glob_may_match(pattern, name) } else { pattern.eq_ignore_ascii_case(name) }
}

fn fits_all(pattern: &[String], node: &[&str]) -> bool {
    pattern.len() >= node.len() && pattern.iter().zip(node).all(|(p, n)| fits(p, n))
}

fn could_name(matcher: &Matcher, pattern: &[String]) -> bool {
    match matcher {
        Matcher::Exact(s) => {
            let node: Vec<&str> = s.split('/').collect();
            pattern.len() == node.len() && fits_all(pattern, &node)
        }
        Matcher::Prefix(s) => fits_all(pattern, &s.trim_end_matches('/').split('/').collect::<Vec<_>>()),
        Matcher::StringPrefix(s) => {
            let node: Vec<&str> = s.split('/').collect();
            let (last, dirs) = node.split_last().unwrap_or((&"", &[]));
            fits_all(pattern, dirs) && pattern.get(dirs.len()).is_some_and(|p| could_start_with(p, last))
        }
        Matcher::Segment(seg) => pattern.iter().any(|p| fits(p, seg)),
        Matcher::Glob(node) => pattern.len() == node.len() && pattern.iter().zip(node).all(|(p, n)| n == "*" || fits(p, n)),
    }
}

/// Whether some name `pattern` matches could begin with `prefix`: the pattern's literal head and
/// the prefix agree as far as both go, and a wildcard after the head can supply the rest.
fn could_start_with(pattern: &str, prefix: &str) -> bool {
    let head_len = pattern.find(['*', '?', '[']).unwrap_or(pattern.len());
    let head = &pattern.as_bytes()[..head_len];
    let prefix = prefix.as_bytes();
    let common = head.len().min(prefix.len());
    head[..common].eq_ignore_ascii_case(&prefix[..common]) && (head.len() >= prefix.len() || head_len < pattern.len())
}

#[cfg(test)]
mod tests {
    use super::super::classify_region;
    use crate::engine::facet::LocalLocus;

    const READS: [(&str, &str); 10] = [
        ("~/.aw\u{17F}/credentials", "~/.aws/credentials"),
        ("~/.\u{17F}sh/id_rsa", "~/.ssh/id_rsa"),
        ("~/.s\u{17F}h/id_ed25519", "~/.ssh/id_ed25519"),
        ("~/.\u{212A}ube/config", "~/.kube/config"),
        ("~/.codex/auth.j\u{17F}on", "~/.codex/auth.json"),
        ("~/.\u{17F}3cfg", "~/.s3cfg"),
        ("~/.git-credential\u{17F}", "~/.git-credentials"),
        ("~/.ne\u{212A}rc", "~/.netrc"),
        ("~/.con\u{FB01}g/gh/hosts.yml", "~/.config/gh/hosts.yml"),
        ("/Users/someone/.aw\u{17F}/credentials", "/Users/someone/.aws/credentials"),
    ];

    #[test]
    fn a_credential_store_spelled_with_a_folding_character_stays_shielded() {
        for (spelled, ascii) in READS {
            assert!(classify_region(ascii).reads_secret, "{ascii} is a shield");
            assert!(classify_region(spelled).reads_secret, "{spelled} reads as {ascii}");
        }
    }

    #[test]
    fn a_frozen_name_spelled_with_a_folding_character_stays_frozen() {
        for path in [".\u{17F}l/config", ".g\u{131}t/hooks/pre-commit", ".j\u{FF4A}/repo/config.toml", "/tmp/r/.\u{17F}l/config"] {
            assert!(classify_region(path).write_locus >= LocalLocus::WorktreeTrusted, "{path}");
        }
    }

    #[test]
    fn a_name_outside_ascii_that_cannot_be_protected_stays_ordinary() {
        for path in ["r\u{E9}sum\u{E9}.txt", "docs/\u{66F8}\u{985E}.md", "\u{1F600}/notes.txt"] {
            assert_eq!(classify_region(path).write_locus, LocalLocus::Worktree, "{path}");
            assert!(!classify_region(path).reads_secret, "{path}");
        }
        assert!(!classify_region("~/\u{66F8}\u{985E}/notes.txt").reads_secret);
    }

    fn shielded_paths() -> Vec<String> {
        super::REGIONS
            .nodes
            .iter()
            .filter(|n| n.applies_here() && (n.role.reads_secret || n.role.write_locus >= LocalLocus::WorktreeTrusted))
            .filter_map(|n| match &n.matcher {
                super::Matcher::Exact(s) if s.starts_with("~/") => Some(s.clone()),
                super::Matcher::Prefix(s) if s.starts_with("~/") => Some(format!("{s}x")),
                super::Matcher::Segment(s) => Some(format!("~/{s}/x")),
                _ => None,
            })
            .collect()
    }

    fn equivalents(word: &str) -> Vec<String> {
        let mut out = Vec::new();
        for (ascii, folded) in
            [("s", "\u{17F}"), ("k", "\u{212A}"), ("fi", "\u{FB01}"), ("fl", "\u{FB02}"), ("ff", "\u{FB00}"), ("st", "\u{FB06}")]
        {
            for (at, _) in word.match_indices(ascii) {
                out.push(format!("{}{folded}{}", &word[..at], &word[at + ascii.len()..]));
            }
        }
        for (at, c) in word.char_indices().filter(|(_, c)| c.is_ascii_alphanumeric()) {
            let lower = c.to_ascii_lowercase();
            let mut forms = vec![];
            if lower.is_ascii_lowercase() {
                let i = u32::from(lower) - u32::from('a');
                forms.extend([0xFF41 + i, 0xFF21 + i, 0x1D41A + i, 0x1D400 + i, 0x24D0 + i, 0x24B6 + i]);
            } else {
                forms.extend([0xFF10 + u32::from(lower) - u32::from('0'), 0x1D7CE + u32::from(lower) - u32::from('0')]);
            }
            for form in forms.into_iter().filter_map(char::from_u32) {
                out.push(format!("{}{form}{}", &word[..at], &word[at + 1..]));
            }
        }
        out
    }

    fn names(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|p| p.to_string()).collect()
    }

    #[test]
    fn a_prefix_can_begin_a_name_a_pattern_matches() {
        use super::could_start_with;
        for (pattern, prefix) in [
            ("abc*", "abc"),
            ("ab*", "abcd"),
            ("abcd", "abc"),
            ("ABC*", "abc"),
            ("*", "anything"),
            ("a?", "abc"),
            ("a[bc]", "abc"),
            ("abc", ""),
            ("", ""),
        ] {
            assert!(could_start_with(pattern, prefix), "{pattern} could begin {prefix}");
        }
        for (pattern, prefix) in [("abc", "abcd"), ("abx*", "abcd"), ("abd", "abc"), ("", "x"), ("b*", "a")] {
            assert!(!could_start_with(pattern, prefix), "{pattern} cannot begin {prefix}");
        }
    }

    #[test]
    fn a_string_prefix_needs_its_folders_and_a_name_that_could_begin_it() {
        use super::{Matcher, could_name};
        let node = Matcher::StringPrefix("dir/last".to_string());
        assert!(could_name(&node, &names(&["dir", "la*"])));
        assert!(could_name(&node, &names(&["dir", "lastly"])));
        assert!(could_name(&node, &names(&["*", "*"])));
        assert!(!could_name(&node, &names(&["other", "last"])));
        assert!(!could_name(&node, &names(&["dir", "zzz"])));
        assert!(!could_name(&node, &names(&["dir"])));
    }

    /// The role handed back is the most protective of the nodes a wildcard-spelled path could name,
    /// not whichever the table lists last.
    #[test]
    fn the_more_protective_of_two_roles_wins_whichever_comes_first() {
        use super::super::Frozen;
        use super::{Role, most_protective};
        let key = |r: Role| (r.reads_secret, r.write_locus, r.read_locus);
        let role = |read, write, reads_secret| Role {
            read_locus: read,
            write_locus: write,
            rebind_locus: write,
            reads_secret,
            frozen: Frozen::Nothing,
        };
        let (user, machine, device) = (LocalLocus::User, LocalLocus::Machine, LocalLocus::Device);
        let (low, high) = (role(user, machine, false), role(device, device, false));
        assert_eq!(key(most_protective(low, high)), key(high));
        assert_eq!(key(most_protective(high, low)), key(high));
        let (secret, louder) = (role(machine, machine, true), role(device, device, false));
        assert_eq!(key(most_protective(secret, louder)), key(secret));
        assert_eq!(key(most_protective(louder, secret)), key(secret));
    }

    #[test]
    fn the_role_is_the_most_protective_node_the_path_could_name() {
        use super::{REGIONS, could_name, home_spelling, pattern, shield_role};
        let mut paths = vec![];
        for pre in [
            "", "/", "~/", "/dev/", "/etc/", "/usr/", "/Library/", "~/Library/", "~/.config/", "/proc/", "/System/", "/private/", "/var/",
            "/tmp/", ".git/", "/boot/", "/sys/", "/Volumes/", "/Users/", "/bin/", "/opt/", "~/.local/", "/Applications/", "/run/", "/lib/",
            "/root/", "/home/", "/cores/", "/sbin/",
        ] {
            for tail in ["\u{1F600}", "\u{1F600}/x", "x/\u{1F600}", "\u{1F600}/\u{1F600}", "a\u{1F600}", ".\u{1F600}"] {
                paths.push(format!("{pre}{tail}"));
            }
        }
        for path in paths.iter().map(String::as_str) {
            let spellings: Vec<Vec<String>> = home_spelling::shield_spellings(path).iter().map(|p| pattern(p)).collect();
            let key = |r: super::Role| (r.reads_secret, r.write_locus, r.read_locus);
            let best = REGIONS
                .nodes
                .iter()
                .filter(|n| n.applies_here() && n.fold)
                .filter(|n| spellings.iter().any(|p| could_name(&n.matcher, p)))
                .map(|n| key(n.role))
                .max();
            assert_eq!(shield_role(path).map(key), best, "{path}");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_home_store_whose_parent_folders_are_spelled_outside_ascii_stays_shielded() {
        let home = std::env::var("HOME").unwrap_or_default();
        let Some(user) = home.strip_prefix("/Users/").filter(|u| !u.is_empty() && !u.contains('/')) else { return };
        let parents = [
            format!("/U\u{17F}ers/{user}"),
            format!("/\u{17F}ystem/Volumes/Data/Users/{user}"),
            format!("/System/Volume\u{17F}/Data/U\u{17F}ers/{user}"),
            format!("/Users/{user}"),
        ];
        for store in shielded_paths() {
            let ascii = classify_region(&store);
            for parent in &parents {
                let spelled = format!("{parent}{}", &store[1..]);
                let s = classify_region(&spelled);
                assert!(s.reads_secret >= ascii.reads_secret, "{spelled} reads as {store}");
                assert!(s.write_locus >= ascii.write_locus, "{spelled} writes as {store}");
            }
        }
        for other in
            ["/U\u{17F}ers/someone/notes.txt", "/System/Volumes/Data/U\u{17F}ers/someone/notes.txt", "/\u{17F}ystem/Volumes/Data/.vol/1/2"]
        {
            assert!(classify_region(other).reads_secret || classify_region(other).read_locus > LocalLocus::Machine, "{other}");
        }
    }

    #[test]
    fn the_equivalents_are_found() {
        assert!(shielded_paths().len() > 20);
        assert!(equivalents(".aws").contains(&".aw\u{17F}".to_string()));
        assert!(equivalents(".config").contains(&".con\u{FB01}g".to_string()));
    }

    proptest::proptest! {
        #[test]
        fn a_shielded_name_with_any_equivalent_character_is_never_more_open(which in 0usize..1000, pick in 0usize..1000) {
            let paths = shielded_paths();
            let ascii = &paths[which % paths.len()];
            let (home, rest) = ascii.split_at(2);
            let forms = equivalents(rest);
            proptest::prop_assume!(!forms.is_empty());
            let spelled = format!("{home}{}", forms[pick % forms.len()]);
            let (a, s) = (classify_region(ascii), classify_region(&spelled));
            proptest::prop_assert!(s.reads_secret >= a.reads_secret, "{} reads as {}", spelled, ascii);
            proptest::prop_assert!(s.write_locus >= a.write_locus, "{} writes as {}", spelled, ascii);
        }
    }
}
