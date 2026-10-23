//! Trees read by pax, 7z and diff when they follow links.
//!
//! BSD pax reads a word after the first operand as one more operand, so a late `-P` never stops it
//! following links, and the word is walked as a source too.

use crate::parse::Token;
use crate::policy::consumes_next_value;

use super::Read;

const PAX_VALUED: &[char] = &['B', 'E', 'G', 'T', 'U', 'b', 'f', 'o', 'p', 's', 'x'];

pub(super) fn pax(tokens: &[Token]) -> Option<Read> {
    let mut follow = None;
    let mut write = false;
    let mut extract = false;
    let mut operands = Vec::new();
    let mut i = 1;
    while i < tokens.len() {
        let word = tokens[i].as_str();
        if word == "--" {
            operands.extend(tokens[i + 1..].iter().map(|token| token.as_str().to_string()));
            break;
        }
        if let Some(cluster) = word.strip_prefix('-').filter(|cluster| !cluster.is_empty()) {
            let late = !operands.is_empty();
            if late {
                operands.push(word.to_string());
            }
            let takes_next = pax_letters(cluster, &mut follow, &mut write, &mut extract, late);
            let step = if takes_next && consumes_next_value(tokens.get(i + 1)) { 2 } else { 1 };
            i += step;
            continue;
        }
        operands.push(word.to_string());
        i += 1;
    }
    if follow != Some(true) || !write {
        return None;
    }
    let sources = if extract { operands.split_last().map(|(_, sources)| sources.to_vec()).unwrap_or_default() } else { operands };
    if sources.is_empty() {
        return Some(names_from_stdin());
    }
    Some(Read { trees: sources, places: Vec::new(), unresolved: false })
}

fn pax_letters(cluster: &str, follow: &mut Option<bool>, write: &mut bool, extract: &mut bool, late: bool) -> bool {
    for (idx, ch) in cluster.char_indices() {
        match ch {
            'L' | 'H' => *follow = Some(true),
            'P' if !late => *follow = Some(false),
            'w' => *write = true,
            'r' => *extract = true,
            _ => {}
        }
        if PAX_VALUED.contains(&ch) {
            return cluster[idx + ch.len_utf8()..].is_empty();
        }
    }
    false
}

fn names_from_stdin() -> Read {
    match super::stdin_name_root() {
        Some(root) => Read { trees: vec![root], places: Vec::new(), unresolved: false },
        None => Read { trees: Vec::new(), places: Vec::new(), unresolved: true },
    }
}

pub(super) fn seven_zip(tokens: &[Token]) -> Option<Read> {
    let (standalone, valued) = crate::registry::command_flag_sets("7z").unwrap_or((&[], &[]));
    let mut follow_l = false;
    let mut store_links = false;
    let mut positionals = Vec::new();
    let mut included = Vec::new();
    let mut unchecked = Vec::new();
    let mut i = 1;
    while i < tokens.len() {
        let word = tokens[i].as_str();
        if word == "--" {
            positionals.extend(tokens[i + 1..].iter().map(|token| token.as_str().to_string()));
            break;
        }
        if word.starts_with('-') && word != "-" {
            let lower = word.to_ascii_lowercase();
            follow_l |= lower == "-l";
            store_links |= lower == "-snl";
            match seven_zip_names(word) {
                Some(Named::List { archives }) => unchecked.push(archives),
                Some(Named::Path { path, archives }) => included.push((path, archives)),
                None => {}
            }
            let listed = |flags: &[String]| flags.iter().any(|flag| flag == word);
            let takes = listed(valued) && !listed(standalone) && !word.contains('=') && consumes_next_value(tokens.get(i + 1));
            i += if takes { 2 } else { 1 };
            continue;
        }
        positionals.push(word.to_string());
        i += 1;
    }
    let command = positionals.first()?.to_ascii_lowercase();
    let adds = command == "a" || command == "u";
    let mut named = included.into_iter().filter(|(_, archives)| adds || *archives).map(|(path, _)| path).peekable();
    let lists = unchecked.iter().any(|archives| adds || *archives) || (adds && positionals.iter().skip(1).any(|p| p.starts_with('@')));
    if lists {
        return Some(Read { trees: Vec::new(), places: Vec::new(), unresolved: true });
    }
    if !adds || (!follow_l && store_links) {
        return named
            .peek()
            .is_some()
            .then(|| Read { trees: Vec::new(), places: named.collect(), unresolved: false });
    }
    let mut trees: Vec<String> = positionals.into_iter().skip(2).filter(|path| path != "-").collect();
    if trees.is_empty() {
        trees.push(".".to_string());
    }
    trees.extend(named);
    Some(Read { trees, places: Vec::new(), unresolved: false })
}

enum Named {
    List { archives: bool },
    Path { path: String, archives: bool },
}

/// A `7z` switch naming more to read: `-i` (files to add) or `-ai` (archives to open), then an
/// optional recursion mark (`r`, `r-`, `r0`), then `@list` or `!path`.
fn seven_zip_names(word: &str) -> Option<Named> {
    let lower = word.to_ascii_lowercase();
    let (archives, rest) = match lower.strip_prefix("-ai") {
        Some(rest) => (true, rest),
        None => (false, lower.strip_prefix("-i")?),
    };
    let rest = rest.strip_prefix('r').map_or(rest, |after| after.strip_prefix(['-', '0']).unwrap_or(after));
    let at = word.len() - rest.len();
    match rest.as_bytes().first()? {
        b'@' => Some(Named::List { archives }),
        b'!' => Some(Named::Path { path: word[at + 1..].to_string(), archives }),
        _ => None,
    }
}

struct DiffScan<'a> {
    recursive: bool,
    noderef: bool,
    trees: Vec<String>,
    standalone: &'a [String],
    valued: &'a [String],
}

pub(super) fn diff(tokens: &[Token]) -> Option<Read> {
    let (standalone, valued) = crate::registry::command_flag_sets("diff").unwrap_or((&[], &[]));
    let mut scan = DiffScan { recursive: false, noderef: false, trees: Vec::new(), standalone, valued };
    let mut i = 1;
    while i < tokens.len() {
        let word = tokens[i].as_str();
        if word == "--" {
            scan.trees
                .extend(tokens[i + 1..].iter().map(|token| token.as_str().to_string()).filter(|path| path != "-"));
            break;
        }
        if let Some(step) = diff_flag(word, tokens.get(i + 1), &mut scan) {
            i += step;
            continue;
        }
        if word != "-" {
            scan.trees.push(word.to_string());
        }
        i += 1;
    }
    (scan.recursive && !scan.noderef).then_some(Read { trees: scan.trees, places: Vec::new(), unresolved: false })
}

fn diff_flag(word: &str, next: Option<&Token>, scan: &mut DiffScan<'_>) -> Option<usize> {
    if let Some(body) = word.strip_prefix("--") {
        let name = body.split_once('=').map(|(name, _)| name).unwrap_or(body);
        scan.recursive |= name == "recursive";
        scan.noderef |= name == "no-dereference";
        let dashed = format!("--{name}");
        let valued = scan.valued.iter().any(|flag| flag == &dashed);
        if valued && !body.contains('=') && consumes_next_value(next) {
            remember_compared(name, next.map(Token::as_str), &mut scan.trees);
            return Some(2);
        }
        remember_compared(name, body.split_once('=').map(|(_, value)| value), &mut scan.trees);
        return Some(1);
    }
    let cluster = word.strip_prefix('-').filter(|cluster| !cluster.is_empty())?;
    let mut step = 1;
    for (idx, ch) in cluster.char_indices() {
        scan.recursive |= ch == 'r';
        let short = format!("-{ch}");
        if scan.valued.iter().any(|flag| flag == &short) {
            if cluster[idx + ch.len_utf8()..].is_empty() && consumes_next_value(next) {
                step = 2;
            }
            break;
        }
        if !scan.standalone.iter().any(|flag| flag == &short) {
            break;
        }
    }
    Some(step)
}

fn remember_compared(name: &str, value: Option<&str>, trees: &mut Vec<String>) {
    if !matches!(name, "from-file" | "to-file") {
        return;
    }
    if let Some(path) = value.filter(|path| !path.is_empty() && *path != "-") {
        trees.push(path.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(words: &[&str]) -> Vec<Token> {
        words.iter().map(|word| Token::from_raw((*word).to_string())).collect()
    }

    #[test]
    fn pax_walks_the_members_and_not_the_archive() {
        let read = pax(&tokens(&["pax", "-wL", "-f", "out.pax", "sub"])).expect("follow");
        assert_eq!(read.trees, vec!["sub".to_string()]);
        assert!(pax(&tokens(&["pax", "-w", "-L", "-P", "sub"])).is_none());
        let late = pax(&tokens(&["pax", "-wL", "sub", "-P"])).expect("a late -P is an operand");
        assert_eq!(late.trees, vec!["sub".to_string(), "-P".to_string()]);
        let unread = pax(&tokens(&["pax", "-wL"])).expect("names from stdin");
        assert!(unread.unresolved);
    }

    #[test]
    fn seven_zip_follows_on_add_unless_it_stores_links() {
        let read = seven_zip(&tokens(&["7z", "a", "-l", "out.7z", "sub"])).expect("follow");
        assert_eq!(read.trees, vec!["sub".to_string()]);
        let added = seven_zip(&tokens(&["7z", "a", "out.7z"])).expect("current directory");
        assert_eq!(added.trees, vec![".".to_string()]);
        assert!(seven_zip(&tokens(&["7z", "a", "-snl", "out.7z", "sub"])).is_none());
        let both = seven_zip(&tokens(&["7z", "a", "-snl", "-l", "out.7z", "sub"])).expect("follow");
        assert_eq!(both.trees, vec!["sub".to_string()]);
    }

    #[test]
    fn diff_walks_both_directories_unless_told_not_to_follow() {
        let read = diff(&tokens(&["diff", "-ruN", "empty", "sub"])).expect("follow");
        assert_eq!(read.trees, vec!["empty".to_string(), "sub".to_string()]);
        assert!(diff(&tokens(&["diff", "-r", "--no-dereference", "sub", "empty"])).is_none());
        assert!(diff(&tokens(&["diff", "sub", "empty"])).is_none());
        let spaced = diff(&tokens(&["diff", "-r", "-U", "3", "sub", "empty"])).expect("follow");
        assert_eq!(spaced.trees, vec!["sub".to_string(), "empty".to_string()]);
    }

    #[test]
    fn pax_copy_reads_the_source_and_not_the_destination() {
        let read = pax(&tokens(&["pax", "-rwL", "src", "dest"])).expect("copy");
        assert_eq!(read.trees, vec!["src".to_string()]);
    }

    #[test]
    fn seven_zip_reads_only_the_members_named_after_options_end() {
        let read = seven_zip(&tokens(&["7z", "a", "-l", "out.7z", "--", "sub"])).expect("follow");
        assert_eq!(read.trees, vec!["sub".to_string()]);
    }

    #[test]
    fn diff_reads_operands_and_the_files_an_option_names() {
        let ended = diff(&tokens(&["diff", "-r", "--", "sub", "empty"])).expect("follow");
        assert_eq!(ended.trees, vec!["sub".to_string(), "empty".to_string()]);
        let recursive = diff(&tokens(&["diff", "--recursive", "sub", "empty"])).expect("follow");
        assert_eq!(recursive.trees, vec!["sub".to_string(), "empty".to_string()]);
        assert!(diff(&tokens(&["diff", "--brief", "sub", "empty"])).is_none());
        let from = diff(&tokens(&["diff", "-r", "--from-file", "sub", "empty"])).expect("follow");
        assert_eq!(from.trees, vec!["sub".to_string(), "empty".to_string()]);
        let dash = diff(&tokens(&["diff", "-r", "--from-file=-", "empty"])).expect("follow");
        assert_eq!(dash.trees, vec!["empty".to_string()]);
    }
}
