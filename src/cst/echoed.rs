//! The path that stands for the items `echo`, `printf` or `ls -d` hands a consumer such as `xargs`.
//!
//! These print their arguments, and `xargs` splits what they print at blanks and reads quotes and
//! backslashes, so each blank-separated piece is an item of its own: `echo "a /etc/hosts"` hands
//! over `/etc/hosts`. The item that stands for them all is the worst piece by where it may be read
//! or written, not by whether it may be read: `/etc/hosts` and `.git` read fine and may not be
//! written. Between pieces that are equally placed, one with write-frozen metadata below it wins,
//! so `echo x . | xargs chmod -R 755` is judged on `.`. A piece xargs would rewrite (quotes,
//! backslashes) or a `printf` format that builds paths from pieces stands for no known path.

use crate::engine::facet::LocalLocus;
use crate::engine::resolve::{frozen_reach, frozen_walk, locus};

/// The item standing for what `name` prints given `args`: `sc_item` when nothing printed reaches
/// past the worktree, `None` when no path stands for it.
/// Several pieces where one may not be read stand for no one path: the unreadable one is the worst
/// read and another may be the worst write.
pub(crate) fn printed_item(name: &str, args: &[&str]) -> Option<String> {
    let printed: Vec<&str> = match name {
        "printf" => printf_items(args)?,
        "ls" => ls_operands(args),
        _ => echo_items(args),
    };
    if printed.iter().any(|a| a.contains(['\'', '"', '\\'])) {
        return None;
    }
    let pieces: Vec<&str> = printed.iter().flat_map(|a| a.split_whitespace()).collect();
    let unreadable = |p: &&str| !crate::engine::resolve::read_content_verdict(p).is_allowed();
    let unwritable = |p: &&str| !crate::engine::resolve::write_target_verdict(p).is_allowed();
    let reaches = |p: &&str| !frozen_reach::read_through(p).is_empty();
    if pieces.len() > 1 && (pieces.iter().any(unreadable) || (pieces.iter().any(unwritable) && pieces.iter().any(reaches))) {
        return None;
    }
    let rank =
        |p: &&str| (unreadable(p), unwritable(p), reaches(p), locus::read_locus(p).max(locus::write_locus(p)).max(locus::rebind_locus(p)));
    Some(pieces.into_iter().max_by_key(rank).map_or_else(|| "sc_item".to_string(), str::to_string))
}

pub(crate) fn ls_prints_operands(args: &[&str]) -> bool {
    ls_flags(args).any(|f| f == "--directory" || (!f.starts_with("--") && f[1..].contains('d')))
}

pub(crate) fn ls_item(args: &[&str]) -> Option<String> {
    let hidden =
        ls_flags(args).any(|f| matches!(f, "--all" | "--almost-all") || (!f.starts_with("--") && f[1..].contains(['a', 'A', 'f'])));
    let follow = ls_flags(args).any(|f| f == "--dereference" || (!f.starts_with("--") && f[1..].contains(['L', 'H'])));
    let walk = frozen_walk::Walk::items(hidden, follow);
    for base in ls_operands(args) {
        if locus::read_locus(base) > LocalLocus::Worktree
            || locus::write_locus(base) > LocalLocus::Worktree
            || !frozen_walk::reach_below(base, walk).is_empty()
        {
            return None;
        }
    }
    Some("sc_item".to_string())
}

fn echo_items<'a>(args: &[&'a str]) -> Vec<&'a str> {
    let flags = args
        .iter()
        .take_while(|a| {
            a.starts_with('-')
                && a.len() > 1
                && a[1..].chars().all(|c| matches!(c, 'n' | 'e' | 'E'))
                && crate::engine::resolve::write_target_verdict(a).is_allowed()
        })
        .count();
    args[flags..].to_vec()
}

fn ls_flags<'a>(args: &'a [&'a str]) -> impl Iterator<Item = &'a str> {
    args.iter().copied().take_while(|a| *a != "--").filter(|a| a.starts_with('-') && a.len() > 1)
}

fn ls_operands<'a>(args: &[&'a str]) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut after_dd = false;
    for &a in args {
        if !after_dd && a == "--" {
            after_dd = true;
        } else if after_dd || !a.starts_with('-') {
            out.push(a);
        }
    }
    if out.is_empty() { vec!["."] } else { out }
}

/// What `printf` prints: the format when it has no directive, or the arguments when the format
/// prints each of them on its own (`%s\n`). Any other format can join arguments into a new path.
fn printf_items<'a>(args: &[&'a str]) -> Option<Vec<&'a str>> {
    let (dashed, args) = match args.strip_prefix(&["--"]) {
        Some(rest) => (true, rest),
        None => (false, args),
    };
    let (format, rest) = args.split_first()?;
    if !dashed && format.starts_with('-') {
        return None;
    }
    if !format.contains(['%', '\\']) {
        return rest.is_empty().then(|| vec![*format]);
    }
    matches!(*format, "%s\\n" | "%s\n" | "%s " | "%s\\0").then(|| rest.to_vec())
}

#[cfg(test)]
#[path = "echoed_tests.rs"]
mod tests;
