//! Paths a searcher reads when it follows links. The valued options are the command's own flag
//! table, so a value is not mistaken for the pattern and the working directory is what gets walked.

use crate::parse::Token;
use crate::policy::consumes_next_value;

struct Kind {
    follow_shorts: &'static [char],
    pattern_shorts: &'static [char],
    follow: &'static [&'static str],
    nofollow: &'static [&'static str],
    pattern: &'static [&'static str],
    files: &'static [&'static str],
    optional_number: &'static [&'static str],
}

const RG: Kind = Kind {
    follow_shorts: &['L'],
    pattern_shorts: &['e', 'f'],
    follow: &["follow"],
    nofollow: &[],
    pattern: &["regexp", "file"],
    files: &["files"],
    optional_number: &[],
};

const AG: Kind = Kind {
    follow_shorts: &['f'],
    pattern_shorts: &[],
    follow: &["follow"],
    nofollow: &["nofollow", "no-follow"],
    pattern: &[],
    files: &[],
    optional_number: &["A", "B", "C", "after", "before", "context"],
};

struct Scan {
    positionals: Vec<String>,
    follow: Option<bool>,
    leading: Leading,
    closed: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Leading {
    DropFirst,
    Keep,
}

pub(super) fn rg(tokens: &[Token]) -> Option<Vec<String>> {
    walked(tokens, "rg", &RG)
}

pub(super) fn ag(tokens: &[Token]) -> Option<Vec<String>> {
    walked(tokens, "ag", &AG)
}

fn walked(tokens: &[Token], cmd: &str, kind: &Kind) -> Option<Vec<String>> {
    let (standalone, valued) = crate::registry::command_flag_sets(cmd).unwrap_or((&[], &[]));
    let mut scan = Scan { positionals: Vec::new(), follow: None, leading: Leading::DropFirst, closed: false };
    let mut i = 1;
    while i < tokens.len() {
        let word = tokens[i].as_str();
        if word == "--" {
            scan.positionals.extend(tokens[i + 1..].iter().map(|token| token.as_str().to_string()));
            break;
        }
        if let Some(body) = word.strip_prefix("--") {
            let name = body.split('=').next().unwrap_or(body);
            match classify(name, standalone, valued) {
                Class::Valued(flag) => {
                    note(flag, kind, &mut scan);
                    if !body.contains('=') {
                        take_value(flag, kind, tokens, &mut i);
                    }
                }
                Class::Plain(flag) => note(flag, kind, &mut scan),
                Class::Unknown => scan.closed = true,
            }
        } else if let Some(cluster) = word.strip_prefix('-').filter(|cluster| !cluster.is_empty()) {
            scan_cluster(cluster, kind, standalone, valued, tokens, &mut scan, &mut i);
        } else {
            scan.positionals.push(word.to_string());
        }
        i += 1;
    }
    paths(scan)
}

fn paths(scan: Scan) -> Option<Vec<String>> {
    if scan.follow != Some(true) {
        return None;
    }
    let mut paths = scan.positionals;
    if scan.leading == Leading::DropFirst && !paths.is_empty() {
        paths.remove(0);
    }
    if scan.closed || paths.is_empty() {
        paths.push(".".to_string());
    }
    Some(paths)
}

enum Class<'a> {
    Valued(&'a str),
    Plain(&'a str),
    Unknown,
}

fn classify<'a>(name: &str, standalone: &'a [String], valued: &'a [String]) -> Class<'a> {
    let valued_hit = named(valued, name);
    let plain_hit = named(standalone, name);
    match (valued_hit, plain_hit) {
        (Found::Exact(_), Found::Exact(_)) => Class::Unknown,
        (Found::Exact(flag), _) => Class::Valued(flag),
        (_, Found::Exact(flag)) => Class::Plain(flag),
        (Found::Prefix(flag), Found::None) => Class::Valued(flag),
        (Found::None, Found::Prefix(flag)) => Class::Plain(flag),
        _ => Class::Unknown,
    }
}

enum Found<'a> {
    Exact(&'a str),
    Prefix(&'a str),
    None,
    Many,
}

fn named<'a>(flags: &'a [String], name: &str) -> Found<'a> {
    let dashed = format!("--{name}");
    if let Some(flag) = flags.iter().find(|flag| flag.as_str() == dashed) {
        return Found::Exact(flag);
    }
    if name.len() < 3 {
        return Found::None;
    }
    let mut found = None;
    for flag in flags {
        let Some(full) = flag.strip_prefix("--") else {
            continue;
        };
        if full.starts_with(name) {
            if found.is_some() {
                return Found::Many;
            }
            found = Some(flag.as_str());
        }
    }
    found.map_or(Found::None, Found::Prefix)
}

fn note(flag: &str, kind: &Kind, scan: &mut Scan) {
    let name = flag.strip_prefix("--").unwrap_or(flag);
    if kind.nofollow.contains(&name) {
        scan.follow = Some(false);
    } else if kind.follow.contains(&name) {
        scan.follow = Some(true);
    }
    if kind.pattern.contains(&name) || kind.files.contains(&name) {
        scan.leading = Leading::Keep;
    }
}

fn scan_cluster(cluster: &str, kind: &Kind, standalone: &[String], valued: &[String], tokens: &[Token], scan: &mut Scan, i: &mut usize) {
    let chars: Vec<char> = cluster.chars().collect();
    for (idx, ch) in chars.iter().enumerate() {
        if kind.follow_shorts.contains(ch) {
            scan.follow = Some(true);
        }
        if kind.pattern_shorts.contains(ch) {
            scan.leading = Leading::Keep;
        }
        let short = format!("-{ch}");
        let takes_value = valued.iter().any(|flag| flag == &short);
        let known = takes_value || standalone.iter().any(|flag| flag == &short);
        if !known {
            scan.closed = true;
            return;
        }
        if takes_value {
            if idx + 1 == chars.len() {
                take_value(&short, kind, tokens, i);
            }
            return;
        }
    }
}

fn take_value(flag: &str, kind: &Kind, tokens: &[Token], i: &mut usize) {
    let next = tokens.get(*i + 1);
    let name = flag.trim_start_matches('-');
    if kind.optional_number.contains(&name) {
        if next.is_some_and(|token| is_integer(token.as_str())) {
            *i += 1;
        }
        return;
    }
    if consumes_next_value(next) {
        *i += 1;
    }
}

fn is_integer(word: &str) -> bool {
    let digits = word.strip_prefix('-').unwrap_or(word);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn every_valued_flag_is_consumed_when_ripgrep_follows(n in 0..64usize) {
            let valued = crate::registry::command_valued_flags("rg").expect("rg flags");
            prop_assume!(!valued.is_empty());
            let flag = valued[n % valued.len()].as_str();
            let tokens = ["rg", "-L", flag, "zz", "needle"].map(|word| Token::from_raw(word.to_string()));
            let roots = rg(&tokens).unwrap_or_default();
            let supplies_pattern = matches!(flag, "-e" | "-f" | "--regexp" | "--file");
            if supplies_pattern {
                prop_assert_eq!(roots, vec!["needle".to_string()]);
            } else {
                prop_assert!(roots.iter().any(|root| root == "."), "{} walked {:?}", flag, roots);
                prop_assert!(!roots.iter().any(|root| root == "zz" || root == "needle"), "{} walked {:?}", flag, roots);
            }
        }

        #[test]
        fn every_valued_flag_is_consumed_when_ag_follows(n in 0..64usize) {
            let valued = crate::registry::command_valued_flags("ag").expect("ag flags");
            prop_assume!(!valued.is_empty());
            let flag = valued[n % valued.len()].as_str();
            let tokens = ["ag", "-f", flag, "zz", "needle"].map(|word| Token::from_raw(word.to_string()));
            let roots = ag(&tokens).unwrap_or_default();
            let optional = matches!(flag, "-A" | "-B" | "-C" | "--after" | "--before" | "--context");
            if optional {
                prop_assert_eq!(roots, vec!["needle".to_string()]);
            } else {
                prop_assert!(roots.iter().any(|root| root == "."), "{} walked {:?}", flag, roots);
                prop_assert!(!roots.iter().any(|root| root == "zz" || root == "needle"), "{} walked {:?}", flag, roots);
            }
        }
    }

    #[test]
    fn an_unknown_option_still_walks_the_working_directory() {
        let tokens = ["rg", "-L", "--made-up", "x"].map(|word| Token::from_raw(word.to_string()));
        let roots = rg(&tokens).expect("follow");
        assert!(roots.iter().any(|root| root == "."), "{roots:?}");
    }
}
