//! Commands that follow a symlink while they read, judged where the link leads.
//!
//! `cp -RL` and `find -L` already walk. An archiver or a searcher whose own flag follows links
//! (`tar -h`, `cpio -oL`, `zip` unless `-y`, `rg -L`, `ag -f`) reads a secret through a link
//! committed in the tree. The walk is the one those copies use.

mod search;

use crate::engine::resolve::frozen_reach::read_through;
use crate::engine::resolve::frozen_walk::on_disk;
use crate::engine::resolve::locus::UNKNOWABLE_ITEM;
use crate::engine::resolve::read_tree_verdict;
use crate::parse::Token;
use crate::verdict::Verdict;

struct Read {
    trees: Vec<String>,
    places: Vec<String>,
}

pub(crate) fn follows_a_secret(tokens: &[Token]) -> bool {
    let Some(read) = roots(tokens) else {
        return false;
    };
    read.places.iter().any(|path| read_tree_verdict(path) == Verdict::Denied) || read.trees.iter().any(|root| tree_holds_a_secret(root))
}

fn tree_holds_a_secret(root: &str) -> bool {
    read_tree_verdict(root) == Verdict::Denied || read_through(root).iter().any(|path| read_tree_verdict(path) == Verdict::Denied)
}

fn roots(tokens: &[Token]) -> Option<Read> {
    let name = crate::registry::canonical_name(tokens.first()?.command_name());
    match name {
        "tar" => tar_roots(tokens),
        "cpio" => cpio_roots(tokens).map(trees_only),
        "zip" => zip_roots(tokens).map(trees_only),
        "rg" => search::rg(tokens).map(trees_only),
        "ag" => search::ag(tokens).map(trees_only),
        _ => None,
    }
}

fn trees_only(trees: Vec<String>) -> Read {
    Read { trees, places: Vec::new() }
}

fn zip_roots(tokens: &[Token]) -> Option<Vec<String>> {
    let stores_links = tokens.iter().skip(1).take_while(|token| token.as_str() != "--").any(|token| {
        let word = token.as_str();
        word == "--symlinks" || word.starts_with("--symlinks=") || (word.starts_with('-') && !word.starts_with("--") && word.contains('y'))
    });
    if stores_links {
        return None;
    }
    let mut positionals = Vec::new();
    let mut i = 1;
    while i < tokens.len() {
        let word = tokens[i].as_str();
        if word == "--" {
            positionals.extend(tokens[i + 1..].iter().map(|token| token.as_str().to_string()));
            break;
        }
        if word.starts_with("--") {
            if !word.contains('=')
                && matches!(word, "--temp-path" | "--suffixes" | "--exclude" | "--include" | "--password" | "--split-size")
            {
                i += 1;
            }
        } else if word.starts_with('-') && word != "-" {
            let cluster = &word[1..];
            if cluster.chars().count() == 1 && matches!(cluster, "b" | "n" | "x" | "i" | "P" | "s" | "t" | "d") {
                i += 1;
            }
        } else {
            positionals.push(word.to_string());
        }
        i += 1;
    }
    Some(positionals.into_iter().skip(1).filter(|path| path != "-").collect())
}

fn cpio_roots(tokens: &[Token]) -> Option<Vec<String>> {
    let follows = tokens.iter().skip(1).take_while(|token| token.as_str() != "--").any(|token| {
        let word = token.as_str();
        word == "--dereference" || (word.starts_with('-') && !word.starts_with("--") && word.contains('L'))
    });
    if !follows {
        return None;
    }
    let item = crate::pathctx::stdin_item_repr()?;
    tree_stood_for(&item).map(|dir| vec![dir])
}

fn tree_stood_for(item: &str) -> Option<String> {
    if item.contains(UNKNOWABLE_ITEM) {
        return None;
    }
    if let Some(dir) = placeholder_dir(item) {
        return Some(dir);
    }
    let placed = on_disk(item)?;
    std::fs::metadata(placed).ok().filter(|meta| meta.is_dir()).map(|_| item.to_string())
}

fn placeholder_dir(item: &str) -> Option<String> {
    let parent = if item == "sc_item" || item == "f" { "" } else { item.strip_suffix("/sc_item").or_else(|| item.strip_suffix("/f"))? };
    Some(if parent.is_empty() { ".".into() } else { parent.to_string() })
}

fn tar_roots(tokens: &[Token]) -> Option<Read> {
    let mut follow = false;
    let mut want_archive = false;
    let mut archive_taken = false;
    let mut dir: Option<String> = None;
    let mut places = Vec::new();
    let mut trees = Vec::new();
    let mut i = 1;
    while i < tokens.len() {
        let word = tokens[i].as_str();
        if let Some((path, step)) = tar_directory(word, tokens.get(i + 1).map(Token::as_str), dir.as_deref()) {
            dir = Some(path.clone());
            places.push(path);
            i += step;
            continue;
        }
        if let Some(long) = word.strip_prefix("--") {
            note_tar_long(long, &mut follow, &mut want_archive, &mut archive_taken);
        } else if let Some(cluster) = word.strip_prefix('-').filter(|cluster| !cluster.is_empty()) {
            follow |= cluster.contains('h');
            want_archive |= cluster.contains('f');
        } else if i == 1 && word.chars().all(|c| c.is_ascii_alphabetic()) {
            follow |= word.contains('h');
            want_archive |= word.contains('f');
        } else if want_archive && !archive_taken {
            archive_taken = true;
        } else if word != "-" {
            trees.push(crate::engine::resolve::tar_bound(dir.as_deref(), word));
        }
        i += 1;
    }
    follow.then_some(Read { trees, places })
}

fn tar_directory(word: &str, next: Option<&str>, dir: Option<&str>) -> Option<(String, usize)> {
    let bound = |value: &str| crate::engine::resolve::tar_bound(dir, value);
    if word == "-C" || word == "--directory" {
        return Some((bound(next?), 2));
    }
    word.strip_prefix("--directory=")
        .or_else(|| word.strip_prefix("-C").filter(|value| !value.is_empty()))
        .map(|value| (bound(value), 1))
}

fn note_tar_long(long: &str, follow: &mut bool, want_archive: &mut bool, archive_taken: &mut bool) {
    let name = long.split('=').next().unwrap_or(long);
    *follow |= name.len() >= 3 && "dereference".starts_with(name);
    if name.len() >= 3 && "file".starts_with(name) && !name.starts_with("files") {
        if long.contains('=') {
            *archive_taken = true;
        } else {
            *want_archive = true;
        }
    }
}
