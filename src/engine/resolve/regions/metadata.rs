//! The write-frozen metadata directories (`.git`, `.jj`, `.hg`, `.sl`, `.envrc`) win over any
//! admit that would otherwise outrank them by specificity.
//!
//! A segment is the least specific matcher, so `/tmp/` out-ranked `.jj` and a write to
//! `/tmp/r/.jj/repo/config.toml` came out as scratch. The VCS reads that config on its next
//! command against that repository wherever the repository lives, so where the directory sits
//! does not make planting it any less an execution. The same holds for a glob that the shell
//! expands onto one of these names (`.j?/repo/config.toml`).

use super::{Matcher, REGIONS, Role, current_os};
use crate::engine::facet::LocalLocus;

pub(super) fn frozen_metadata_wins(path: &str, role: Role) -> Role {
    let fold = current_os() == "macos";
    let frozen = REGIONS
        .nodes
        .iter()
        .filter(|n| n.applies_here() && n.role.write_locus == LocalLocus::WorktreeTrusted)
        .find(|n| {
            let Matcher::Segment(seg) = &n.matcher else { return false };
            path.split('/').any(|c| component_may_name(c, seg, n.fold && fold))
        });
    match frozen {
        Some(node) if node.role.write_locus > role.write_locus => node.role,
        _ => role,
    }
}

fn component_may_name(component: &str, name: &str, fold: bool) -> bool {
    if component.contains(['*', '?', '[']) {
        return glob_may_match(component, name);
    }
    if fold { component.eq_ignore_ascii_case(name) } else { component == name }
}

fn glob_may_match(pattern: &str, name: &str) -> bool {
    if name.starts_with('.') && !pattern.starts_with('.') {
        return false;
    }
    let pattern = collapse_brackets(&pattern.to_ascii_lowercase());
    let name: Vec<char> = name.to_ascii_lowercase().chars().collect();
    wildcard_match(&pattern, &name)
}

fn collapse_brackets(pattern: &str) -> Vec<char> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' {
            let body = i + 1 + usize::from(matches!(chars.get(i + 1), Some('!' | '^')));
            let close = chars.iter().skip(body + 1).position(|c| *c == ']').map(|p| p + body + 1);
            if let Some(close) = close {
                out.push('?');
                i = close + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn wildcard_match(p: &[char], n: &[char]) -> bool {
    let (mut pi, mut ni, mut star, mut mark) = (0, 0, None, 0);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|c| *c == '*')
}

#[cfg(test)]
mod tests {
    use super::super::classify_region;
    use super::*;
    use proptest::prelude::*;

    const FROZEN: [&str; 5] = [".git", ".jj", ".hg", ".sl", ".envrc"];

    #[test]
    fn metadata_under_temp_is_write_frozen() {
        for dir in [".git", ".jj", ".hg", ".sl"] {
            for base in ["/tmp/r", "/var/tmp/r", "/private/tmp/r", "/tmp"] {
                let path = format!("{base}/{dir}/config");
                assert_eq!(classify_region(&path).write_locus, LocalLocus::WorktreeTrusted, "{path}");
                assert!(classify_region(&path).read_locus <= LocalLocus::WorktreeTrusted, "{path} stays readable");
            }
        }
        assert_eq!(classify_region("/tmp/r/.envrc").write_locus, LocalLocus::WorktreeTrusted);
        assert!(classify_region("/tmp/r/src/a.rs").write_locus < LocalLocus::Worktree);
    }

    #[test]
    fn a_stricter_region_keeps_its_own_answer() {
        let shielded = classify_region("/root/.ssh/.git/config");
        assert!(shielded.reads_secret && shielded.write_locus > LocalLocus::WorktreeTrusted);
    }

    #[test]
    fn globs_that_could_expand_onto_metadata_are_frozen() {
        for path in [
            ".j?/repo/config.toml", ".[j]j/repo/config.toml", ".gi[t]/hooks/pre-commit", ".en?rc", ".*/config", "/tmp/r/.g*/config",
            ".[!x]it/config",
        ] {
            assert_eq!(classify_region(path).write_locus, LocalLocus::WorktreeTrusted, "{path}");
        }
    }

    #[test]
    fn globs_the_shell_cannot_expand_onto_a_dot_name_stay_ordinary() {
        for path in ["target/*", "src/*.rs", "*/config", "build/?", "x[.]git/config", ".gx?/config"] {
            assert_eq!(classify_region(path).write_locus, LocalLocus::Worktree, "{path}");
        }
    }

    #[test]
    fn brackets_collapse_to_one_character() {
        assert_eq!(collapse_brackets(".gi[tx]"), ".gi?".chars().collect::<Vec<_>>());
        assert_eq!(collapse_brackets("[!a]b"), "?b".chars().collect::<Vec<_>>());
        assert_eq!(collapse_brackets("a[]]b"), "a?b".chars().collect::<Vec<_>>());
        assert_eq!(collapse_brackets("a[b"), "a[b".chars().collect::<Vec<_>>());
    }

    proptest! {
        #[test]
        fn a_pattern_masking_any_letters_of_a_frozen_name_matches_it(
            which in 0..FROZEN.len(),
            mask in proptest::collection::vec(0u8..4, 8),
        ) {
            let name = FROZEN[which];
            let pattern: String = name
                .chars()
                .enumerate()
                .map(|(i, c)| match (i, mask[i % mask.len()]) {
                    (0, _) | (_, 0) => c.to_string(),
                    (_, 1) => "?".to_string(),
                    (_, 2) => format!("[{c}]"),
                    _ => "*".to_string(),
                })
                .collect();
            prop_assert!(component_may_name(&pattern, name, false), "{} should match {}", pattern, name);
        }

        #[test]
        fn a_literal_component_matches_only_itself(component in "[a-z.]{1,6}", which in 0..FROZEN.len()) {
            let name = FROZEN[which];
            prop_assert_eq!(component_may_name(&component, name, false), component == name);
        }
    }
}
