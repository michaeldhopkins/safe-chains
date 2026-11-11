//! `pax -r -w SOURCE… DEST` copies each source hierarchy to `DEST/SOURCE`, so the last operand is
//! written and every source lands below it under its own path. Without `-r -w` the operands are
//! source hierarchies to archive (`-w`) or patterns to list, and are read.
//!
//! BSD pax (the one macOS ships) stops reading options at the first operand, so in
//! `pax -w ./src -x ~/.ssh` the words `-x` and `~/.ssh` are two more hierarchies to archive. A pax
//! built on glibc's getopt reads them as options instead. Both readings are judged: every word from
//! the first operand on is also gated as a tree read.

use super::{Role, gate};
use crate::engine::resolve::frozen_reach::{frozen_below, read_through};
use crate::parse::Token;

const VALUED: &[char] = &['B', 'E', 'G', 'T', 'U', 'b', 'f', 'o', 'p', 's', 'x'];

pub(super) fn pax_mode(tokens: &[Token]) -> bool {
    let mut letters = String::new();
    let mut operands: Vec<&str> = Vec::new();
    let mut after_first_operand: &[Token] = &[];
    let mut words = tokens.iter().enumerate().skip(1).map(|(at, token)| (at, token.as_str()));
    while let Some((at, word)) = words.next() {
        if word == "--" {
            operands.extend(words.by_ref().map(|(_, word)| word));
            break;
        }
        let Some(cluster) = word.strip_prefix('-').filter(|c| !c.is_empty()) else {
            if operands.is_empty() {
                after_first_operand = &tokens[at..];
            }
            operands.push(word);
            continue;
        };
        for (offset, c) in cluster.char_indices() {
            letters.push(c);
            if VALUED.contains(&c) {
                let glued = &cluster[offset + c.len_utf8()..];
                let value = if glued.is_empty() { words.next().map(|(_, word)| word) } else { Some(glued) };
                if c == 'f' && value.is_some_and(|v| gate(Role::Write, v)) {
                    return true;
                }
                break;
            }
        }
    }
    if after_first_operand.iter().any(|word| word.as_str() != "--" && gate(Role::ReadTree, word.as_str())) {
        return true;
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
