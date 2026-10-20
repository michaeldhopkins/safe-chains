//! `pax -r -w SOURCE… DEST` copies each source hierarchy to `DEST/SOURCE`, so the last operand is
//! written and every source lands below it under its own path. Without `-r -w` the operands are
//! source hierarchies to archive (`-w`) or patterns to list, and are read.

use super::{Role, gate};
use crate::engine::resolve::frozen_reach::{frozen_below, read_through};
use crate::parse::Token;

const VALUED: &[char] = &['B', 'E', 'G', 'T', 'U', 'b', 'f', 'o', 'p', 's', 'x'];

pub(super) fn pax_mode(tokens: &[Token]) -> bool {
    let mut letters = String::new();
    let mut operands: Vec<&str> = Vec::new();
    let mut words = tokens.iter().skip(1).map(Token::as_str);
    while let Some(word) = words.next() {
        if word == "--" {
            operands.extend(words.by_ref());
            break;
        }
        let Some(cluster) = word.strip_prefix('-').filter(|c| !c.is_empty()) else {
            operands.push(word);
            continue;
        };
        for (at, c) in cluster.char_indices() {
            letters.push(c);
            if VALUED.contains(&c) {
                let glued = &cluster[at + c.len_utf8()..];
                let value = if glued.is_empty() { words.next() } else { Some(glued) };
                if c == 'f' && value.is_some_and(|v| gate(Role::Write, v)) {
                    return true;
                }
                break;
            }
        }
    }
    if !(letters.contains('r') && letters.contains('w')) {
        return operands.iter().any(|p| gate(Role::ReadTree, p));
    }
    let Some((dest, sources)) = operands.split_last() else {
        return false;
    };
    let follows = letters.contains('L');
    gate(Role::Write, dest)
        || frozen_below(dest, true).is_some()
        || sources.iter().any(|source| {
            gate(Role::ReadTree, source)
                || gate(Role::Write, &format!("{}/{source}", dest.trim_end_matches('/')))
                || frozen_below(source, true).is_some()
                || (follows && read_through(source).iter().any(|at| gate(Role::ReadTree, at)))
        })
}
