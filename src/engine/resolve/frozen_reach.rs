//! A recursive operation reaches everything below its operand, not only the operand.
//!
//! `chmod -R 755 .` names the project root, a worktree path, yet walks into `.git/hooks` and makes
//! whatever is there executable; `cp -r other/. .` writes `other/.git` over the project's own. The
//! write-frozen metadata names (`.git`, `.jj`, `.hg`, `.sl`, `.envrc`) are frozen at any depth, so a
//! recursive write is judged as a write to the first one that exists below its operand. The folder
//! is listed at decision time, which races the shell as the glob listing does (HP-5). A tree too
//! large to list within the budget is judged as holding one. With no project root known, nothing is
//! listed and the operand is judged as before.

use std::collections::VecDeque;

use crate::verdict::Verdict;

const MAX_ENTRIES: usize = 20_000;

/// What a walk below an operand turned up.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Reach {
    /// A frozen name that exists, spelled as the operand joined with its path below it.
    Found(String),
    /// A tree that could not be listed in full, stood for by a frozen name directly below it.
    Unlisted(String),
}

impl Reach {
    fn path(&self) -> &str {
        match self {
            Reach::Found(p) | Reach::Unlisted(p) => p,
        }
    }
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

/// Where a recursive copy of `source` into `dest` writes the first frozen name below `source`:
/// `cp -r other/. .` writes `other/.git` as `./.git`. A name that cannot be re-rooted (below a glob
/// source) is returned where it was found, which is frozen itself. A source that is itself a frozen
/// folder, named without a trailing slash, lands under its own name: `rsync -a sub/.git d/` writes
/// `d/.git`.
pub(crate) fn landed(source: &str, dest: &str) -> Option<String> {
    let own = source.rsplit('/').next().filter(|name| !source.ends_with('/') && is_frozen(name));
    if let Some(name) = own {
        return Some(format!("{}/{name}/f", dest.trim_end_matches('/')));
    }
    let reached = frozen_below(source, true)?;
    match reached.path().strip_prefix(source.trim_end_matches('/')) {
        Some(below) if below.starts_with('/') => Some(format!("{}{below}", dest.trim_end_matches('/'))),
        _ => Some(reached.path().to_string()),
    }
}

/// Whether a tree copy of `source` into the last of `operands` (`rsync -a`, `ditto`) writes a
/// frozen name.
pub(crate) fn lands_frozen(source: &str, operands: &[&str]) -> bool {
    let Some(dest) = operands.last() else { return false };
    landed(source, dest).is_some_and(|at| super::write_target_verdict(&at) == Verdict::Denied)
}

/// The verdict of a traversal's inner command (`find -exec`, `fd -x`, `find -delete`) rooted at
/// `base`, `judge` being handed the path each item stands for. When the traversal lists hidden
/// entries and frozen metadata lies below `base`, the command must also pass on a path inside it:
/// `find . -exec chmod +x {} +` reaches `.git/hooks`. A read passes there as anywhere in the
/// project, so only a refusal is taken from that second judgement.
pub(crate) fn judged_over(base: &str, hidden: bool, judge: impl Fn(&str) -> Verdict) -> Verdict {
    let plain = super::locus::traversal_item(base);
    let verdict = judge(&plain);
    if verdict == Verdict::Denied || plain == super::locus::UNKNOWABLE_ITEM {
        return verdict;
    }
    match frozen_below(base, hidden) {
        Some(reach) if judge(reach.path()) == Verdict::Denied => Verdict::Denied,
        _ => verdict,
    }
}

/// The path a pipeline stage's producer (`find … | xargs chmod +x`) stands for: inside the first
/// frozen name below `base` when it lists hidden entries (`find`, `fd -H`), else the plain item.
pub(crate) fn stage_item(name: &str, base: &str, args: &[&str]) -> String {
    let plain = super::locus::traversal_item(base);
    if plain == super::locus::UNKNOWABLE_ITEM {
        return plain;
    }
    frozen_below(base, name == "find" || fd_lists_hidden(args)).map_or(plain, |r| r.path().to_string())
}

/// Whether an `fd` command line lists hidden entries, which fd skips unless told otherwise.
pub(crate) fn fd_lists_hidden(args: &[&str]) -> bool {
    args.iter().any(|a| match a.strip_prefix("--") {
        Some(long) => matches!(long, "hidden" | "unrestricted"),
        None => a.starts_with('-') && a[1..].contains(['H', 'u']),
    })
}

fn unlisted(base: &str) -> Reach {
    Reach::Unlisted(format!("{}/.git/f", base.trim_end_matches('/')))
}

/// The first write-frozen name under the operand `base`. `None` when nothing frozen lies below, or
/// when `base` cannot be placed on disk. Links below `base` are not followed, as `chmod -R`,
/// `cp -r` and `find` do not follow them.
pub(crate) fn frozen_below(base: &str, hidden: bool) -> Option<Reach> {
    if base.contains(['*', '?', '[']) {
        return through_glob(base, hidden);
    }
    let Some(dir) = on_disk(base) else {
        let unplaced = base.contains('$') || base.contains(crate::cst::eval::TAGGED_PREFIX);
        return (unplaced && crate::pathctx::root().is_some_and(|r| r.starts_with('/'))).then(|| unlisted(base));
    };
    let mut queue = VecDeque::from([String::new()]);
    let mut seen = 0usize;
    while let Some(rel) = queue.pop_front() {
        let here = if rel.is_empty() { dir.clone() } else { format!("{dir}/{rel}") };
        let Ok(entries) = std::fs::read_dir(&here) else { continue };
        for entry in entries {
            seen += 1;
            let Some(name) = entry.as_ref().ok().and_then(|e| e.file_name().into_string().ok()).filter(|_| seen <= MAX_ENTRIES) else {
                return Some(unlisted(base));
            };
            if !hidden && name.starts_with('.') {
                continue;
            }
            let below = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            if is_frozen(&name) {
                return Some(Reach::Found(format!("{}/{below}/f", base.trim_end_matches('/'))));
            }
            if entry.ok().and_then(|e| e.file_type().ok()).is_some_and(|t| t.is_dir()) {
                queue.push_back(below);
            }
        }
    }
    None
}

/// A glob operand reaches what each of its matches reaches. Only a pattern in the last component is
/// listed; one higher up names folders that are not listed, so it is judged as unlisted.
fn through_glob(base: &str, hidden: bool) -> Option<Reach> {
    let trimmed = base.trim_end_matches('/');
    let (parent, pattern) = trimmed.rsplit_once('/').map_or((".", trimmed), |(p, l)| (if p.is_empty() { "/" } else { p }, l));
    if parent.contains(['*', '?', '[']) {
        return on_disk(".").map(|_| unlisted(base));
    }
    let Ok(entries) = std::fs::read_dir(on_disk(parent)?) else { return None };
    for entry in entries {
        let Some(name) = entry.ok().and_then(|e| e.file_name().into_string().ok()) else {
            return Some(unlisted(base));
        };
        if name.starts_with('.') && !pattern.starts_with('.') {
            continue;
        }
        match crate::cst::opaque::glob::matches(pattern.as_bytes(), name.as_bytes()) {
            Some(true) => {}
            Some(false) => continue,
            None => return Some(unlisted(base)),
        }
        let frozen = is_frozen(&name);
        let each = if parent == "." { name } else { format!("{}/{name}", parent.trim_end_matches('/')) };
        if frozen {
            return Some(Reach::Found(format!("{each}/f")));
        }
        if let Some(reached) = frozen_below(&each, hidden) {
            return Some(reached);
        }
    }
    None
}

fn is_frozen(name: &str) -> bool {
    let fold = cfg!(target_os = "macos");
    super::regions::metadata::frozen_names()
        .iter()
        .any(|n| if fold { n.eq_ignore_ascii_case(name) } else { *n == name })
}

fn on_disk(base: &str) -> Option<String> {
    let root = crate::pathctx::root().filter(|r| r.starts_with('/'))?;
    if base.contains('$') || base.contains(crate::cst::eval::TAGGED_PREFIX) {
        return None;
    }
    let placed = crate::pathctx::resolve(base);
    if placed.starts_with('/') {
        return Some(placed.into_owned());
    }
    Some(if placed.is_empty() || placed == "." { root } else { format!("{root}/{placed}") })
}

#[cfg(test)]
#[path = "frozen_reach_tests.rs"]
mod tests;
