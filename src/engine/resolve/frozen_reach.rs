//! A recursive operation reaches everything below its operand, not only the operand.
//!
//! `chmod -R 755 .` names the project root, a worktree path, yet walks into `.git/hooks` and makes
//! whatever is there executable; `cp -r other/. .` writes `other/.git` over the project's own. The
//! write-frozen metadata names (`.git`, `.jj`, `.hg`, `.sl`, `.envrc`) are frozen at any depth, so a
//! recursive write is judged as a write to the first one that exists below its operand. The folder
//! is listed at decision time, which races the shell as the glob listing does (HP-5). A tree too
//! large to list within the budget is judged as holding one. With no project root known, nothing is
//! listed and the operand is judged as before. A command that follows links while it walks is also
//! judged where each link out of the tree leads (`frozen_walk`).

use super::frozen_walk::{Skip, Walk, is_frozen, reach_below};
use crate::verdict::Verdict;

/// What a walk below an operand turned up.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Reach {
    /// A frozen name that exists, or a link out of the tree, spelled as the operand joined with its
    /// path below it.
    Found(String),
    /// A tree that could not be listed in full, stood for by a frozen name directly below it.
    Unlisted(String),
}

impl Reach {
    pub(crate) fn path(&self) -> &str {
        match self {
            Reach::Found(p) | Reach::Unlisted(p) => p,
        }
    }
}

/// What a walk has seen. [`Sight::Found`] dominates [`Sight::Unlisted`], so one unreadable
/// directory cannot erase a frozen name listed beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sight {
    Empty,
    Unlisted,
    Found,
}

/// `seen` combined with `next`. Order does not matter: a frozen name stays found.
pub(crate) fn fold_sight(seen: Sight, next: Sight) -> Sight {
    match (seen, next) {
        (Sight::Found, _) | (_, Sight::Found) => Sight::Found,
        (Sight::Unlisted, _) | (_, Sight::Unlisted) => Sight::Unlisted,
        (Sight::Empty, Sight::Empty) => Sight::Empty,
    }
}

pub(crate) fn sight_of(reached: &[Reach]) -> Sight {
    reached
        .iter()
        .fold(Sight::Empty, |seen, reach| fold_sight(seen, if matches!(reach, Reach::Found(_)) { Sight::Found } else { Sight::Unlisted }))
}

/// A walk that stops at the first frozen name may stop before other glob matches only once one
/// is found. An unlisted literal is not that: the shell can still expand onto a match.
pub(crate) fn stops_before_matches(ignore_links: bool, seen: Sight) -> bool {
    ignore_links && seen == Sight::Found
}

/// The operands of a write-role command, followed by the frozen places a recursive one reaches
/// below them. `rm -r .` removes `.git` as `rm -r .git` does, so a frozen name found below a removal
/// counts too; but a removal plants nothing to run, so an unlisted tree is let through for it.
pub(super) fn with_reached(spec: &crate::registry::types::BehaviorSpec, operands: &[&str], tokens: &[crate::parse::Token]) -> Vec<String> {
    let mut all: Vec<String> = operands.iter().map(|p| (*p).to_string()).collect();
    if spec.unbounded_flags.iter().any(|f| super::behavior_flag_present(tokens, f)) {
        let removes = spec.operation == crate::engine::facet::Operation::Destroy;
        let reached = operands.iter().filter_map(|p| frozen_below(p, true));
        all.extend(reached.filter(|r| !removes || matches!(r, Reach::Found(_))).map(|r| r.path().to_string()));
    }
    all
}

/// Where a recursive copy of `source` into `dest` writes what it reaches below `source`:
/// `cp -r other/. .` writes `other/.git` as `./.git`. A name that cannot be re-rooted (below a glob
/// source) is returned where it was found, which is frozen itself. A source that is itself a frozen
/// folder, named without a trailing slash, lands under its own name: `rsync -a sub/.git d/` writes
/// `d/.git`.
pub(crate) fn landed(source: &str, dest: &str, follow: bool) -> Vec<String> {
    let dest = dest.trim_end_matches('/');
    let own = source.rsplit('/').next().filter(|name| !source.ends_with('/') && is_frozen(name));
    if let Some(name) = own {
        return vec![format!("{dest}/{name}/f")];
    }
    let walk = if follow { Walk::follow() } else { Walk::plain(true) };
    reach_below(source, walk)
        .iter()
        .map(|reached| match reached.path().strip_prefix(source.trim_end_matches('/')) {
            Some(below) if below.starts_with('/') => format!("{dest}{below}"),
            _ => reached.path().to_string(),
        })
        .collect()
}

/// The places a copy that follows links (`cp -RL`, `rsync -L`) reads through the links below
/// `source` that lead out of it.
pub(crate) fn read_through(source: &str) -> Vec<String> {
    reach_below(source, Walk::follow()).iter().map(|r| r.path().to_string()).collect()
}

/// Whether a tree copy of `source` into the last of `operands` (`rsync -a`, `ditto`) writes a
/// frozen name, or with `follow` reads where a link below it leads.
pub(crate) fn lands_frozen(source: &str, operands: &[&str], follow: bool) -> bool {
    let Some(dest) = operands.last() else { return false };
    let denied = |at: &String| super::write_target_verdict(at) == Verdict::Denied;
    landed(source, dest, follow).iter().any(denied)
        || (follow && read_through(source).iter().any(|at| super::read_tree_verdict(at) == Verdict::Denied))
}

/// Whether a copy's operand `p` reaches a place it may not: as a tree source (`tree`), by landing a
/// frozen name or reading through a followed link; as the destination (`dest`), by writing through
/// a link it keeps there.
pub(crate) fn copy_reaches(p: &str, operands: &[&str], links: super::link_mode::Links, (tree, dest): (bool, bool)) -> bool {
    (tree && lands_frozen(p, operands, links.follow))
        || (dest && links.dest == super::link_mode::DestLinks::WrittenThrough && writes_through(p))
}

/// Whether a copy that keeps the links to folders in its destination (`rsync -K`) writes through one
/// to a place it may not write.
pub(crate) fn writes_through(dest: &str) -> bool {
    reach_below(dest, Walk::follow())
        .iter()
        .any(|r| super::write_target_verdict(r.path()) == Verdict::Denied)
}

/// The verdict of a traversal's inner command (`find -exec`, `fd -x`, `find -delete`) rooted at
/// `base`, `judge` being handed the path each item stands for. When the traversal lists hidden
/// entries and frozen metadata lies below `base`, the command must also pass on a path inside it:
/// `find . -exec chmod +x {} +` reaches `.git/hooks`. A read passes there as anywhere in the
/// project, so only a refusal is taken from that second judgement.
pub(crate) fn judged_over(base: &str, walk: Walk, judge: impl Fn(&str) -> Verdict) -> Verdict {
    let plain = super::locus::traversal_item(base);
    let verdict = judge(&plain);
    if verdict == Verdict::Denied || plain == super::locus::UNKNOWABLE_ITEM {
        return verdict;
    }
    if reach_below(base, walk).iter().any(|reach| judge(reach.path()) == Verdict::Denied) { Verdict::Denied } else { verdict }
}

/// The path a pipeline stage's producer (`find … | xargs chmod +x`) stands for: inside the first
/// frozen name below `base` when it lists hidden entries (`find`, `fd -H`), else the plain item.
/// When it reaches several places (links out of the tree), no one path stands for them all.
pub(crate) fn stage_item(name: &str, base: &str, args: &[&str]) -> String {
    let plain = super::locus::traversal_item(base);
    if plain == super::locus::UNKNOWABLE_ITEM {
        return plain;
    }
    let find = name == "find";
    let skip = if find { crate::handlers::coreutils::find_skip::skip_of(args) } else { Skip::default() };
    let hidden = find || fd_lists_hidden(args);
    let follow = if find { find_follows(args) } else { fd_follows(args) };
    let walk = Walk { skip: &skip, ..Walk::items(hidden, follow) };
    match reach_below(base, walk).as_slice() {
        [] => plain,
        [one] => one.path().to_string(),
        _ => super::locus::UNKNOWABLE_ITEM.to_string(),
    }
}

/// Whether an `fd` command line lists hidden entries, which fd skips unless told otherwise.
pub(crate) fn fd_lists_hidden(args: &[&str]) -> bool {
    args.iter().any(|a| match a.strip_prefix("--") {
        Some(long) => matches!(long, "hidden" | "unrestricted"),
        None => a.starts_with('-') && a[1..].contains(['H', 'u']),
    })
}

/// Whether an `fd` command line follows links while it walks (`-L`, `--follow`).
pub(crate) fn fd_follows(args: &[&str]) -> bool {
    args.iter().any(|a| match a.strip_prefix("--") {
        Some(long) => long.len() >= 3 && "follow".starts_with(long),
        None => a.starts_with('-') && a[1..].contains('L'),
    })
}

/// Whether a `find` command line follows links: `-L` everywhere, `-H` for its operands, or the
/// `-follow` primary. Looked for in every word, so an inner command's own flag counts too.
pub(crate) fn find_follows(args: &[&str]) -> bool {
    args.iter().any(|a| matches!(*a, "-L" | "-H" | "-follow"))
}

/// The first write-frozen name under the operand `base`, links below it not entered. `None` when
/// nothing frozen lies below, or when `base` cannot be placed on disk.
pub(crate) fn frozen_below(base: &str, hidden: bool) -> Option<Reach> {
    // A found name wins over an unlisted sibling. `rm` keeps only `Found`, so the unlisted
    // entry must not be the one returned when a readable match was also walked.
    let reached = reach_below(base, Walk::plain(hidden));
    let found = reached.iter().position(|r| matches!(r, Reach::Found(_))).unwrap_or(0);
    reached.into_iter().nth(found)
}

#[cfg(test)]
#[path = "frozen_reach_tests.rs"]
mod tests;
