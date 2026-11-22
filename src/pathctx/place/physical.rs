//! Following symbolic links on disk, so a path is placed where the operating system will take it.
//!
//! A link committed in the project (`rc -> ~/.zshrc`) makes `cat rc` read a file outside it, and
//! `link/../x` climbs from the link's target, not from the link. So before a path is judged against
//! the project root, every component that exists is looked up with `lstat`, each link is replaced by
//! its target (relative to the link's own directory), and `..` steps back from the physical place,
//! as the kernel does. Components that do not exist yet are kept as written: nothing below them can
//! be a link.
//!
//! The final component is followed for a read or a write, which open through it, and kept for a
//! rebind (`rm`, `mv`, `ln`), which acts on the link itself, unless the path ends in `/` or `/.`,
//! which the kernel resolves through the link.
//!
//! The lookup happens when the hook decides, not when the shell runs the command. A link created
//! or changed in between (by an earlier command in the same line, or by another process) is not
//! seen. safe-chains refuses `ln -s` to a target outside the project, so a line cannot write a new
//! escape, though it can copy or move a link that already exists under a new name. Another process
//! racing the decision can do the same, and no check made before execution can close that window.
//! Recorded in `docs/design/hard-problems.md`, HP-5.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};

const MAX_LINKS: usize = 40;
const MAX_ENTRIES: usize = 4096;
const MAX_MATCHES: usize = 256;

/// macOS keeps `/etc`, `/tmp` and `/var` as links into `/private`, and `/home` as a link into the
/// data volume, whose root is the same tree as `/`. The region table names the short spellings, so
/// a physical path is written back with them.
const SYSTEM_LINKS: [&str; 3] = ["/private/etc", "/private/tmp", "/private/var"];
const DATA_VOLUME: &str = "/System/Volumes/Data";

/// What one `lstat` found. A link whose target cannot be read as text carries `None`.
#[derive(Clone)]
enum Lookup {
    Missing,
    Other,
    Link(Option<String>),
}

thread_local! {
    static MEMO: RefCell<HashMap<String, Lookup>> = RefCell::new(HashMap::new());
    static EVALUATIONS: Cell<usize> = const { Cell::new(0) };
}

/// Every lookup made while an evaluation is in force is remembered until it ends, so all the checks
/// of one decision read the same disk and a folder is looked up once, however many paths pass it.
pub(in crate::pathctx) fn evaluation_begins() {
    EVALUATIONS.set(EVALUATIONS.get() + 1);
    MEMO.with(|m| m.borrow_mut().clear());
}

pub(in crate::pathctx) fn evaluation_ends() {
    EVALUATIONS.set(EVALUATIONS.get().saturating_sub(1));
    MEMO.with(|m| m.borrow_mut().clear());
}

fn lookup(path: &str) -> Lookup {
    let remember = EVALUATIONS.get() > 0;
    if remember && let Some(known) = MEMO.with(|m| m.borrow().get(path).cloned()) {
        return known;
    }
    let found = match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            Lookup::Link(std::fs::read_link(path).ok().and_then(|t| t.to_str().map(str::to_string)))
        }
        Ok(_) => Lookup::Other,
        Err(_) => Lookup::Missing,
    };
    if remember {
        MEMO.with(|m| m.borrow_mut().insert(path.to_string(), found.clone()));
    }
    found
}

/// A walked path: no link met, or the physical spelling it leads to.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Plain,
    Followed(String),
}

struct Budget {
    links: usize,
    entries: usize,
    matches: usize,
}

/// The physical spelling of the absolute path `raw`, read as the shell hands it over (not yet
/// normalized), or `None` when no link lies on its way. A loop, an unreadable link or a glob whose
/// matches lead to different places yields the unresolvable placeholder.
pub(in crate::pathctx) fn resolve(raw: &str, follow_last: bool) -> Option<String> {
    if !raw.starts_with('/') {
        return None;
    }
    let stripped = crate::engine::resolve::locus::lookup_prefix::strip_lookup_prefixes(raw);
    let raw = stripped.as_deref().unwrap_or(raw);
    let follow_last = follow_last || raw.ends_with('/') || raw.ends_with("/.");
    let mut budget = Budget { links: 0, entries: 0, matches: 0 };
    match walk(Vec::new(), parts(raw, true), follow_last, &mut budget) {
        Outcome::Plain => None,
        Outcome::Followed(real) => (real != crate::pathctx::lexical_join("/", raw)).then_some(real),
    }
}

/// The project root as the kernel sees it, so a path resolved through links is compared with a root
/// resolved the same way.
pub(in crate::pathctx) fn root(root: &str) -> String {
    resolve(root, true).unwrap_or_else(|| root.to_string())
}

fn parts(path: &str, from_shell: bool) -> VecDeque<(String, bool)> {
    path.split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .map(|s| (s.to_string(), from_shell))
        .collect()
}

fn joined(done: &[String]) -> String {
    format!("/{}", done.join("/"))
}

fn unresolved() -> Outcome {
    Outcome::Followed(crate::pathctx::UNRESOLVED_CWD.to_string())
}

fn system_spelling(path: String) -> String {
    if let Some(rest) = path.strip_prefix(DATA_VOLUME).filter(|rest| rest.starts_with('/')) {
        return system_spelling(rest.to_string());
    }
    for long in SYSTEM_LINKS {
        if path == long || path.starts_with(&format!("{long}/")) {
            return path["/private".len()..].to_string();
        }
    }
    path
}

fn walk(mut done: Vec<String>, mut todo: VecDeque<(String, bool)>, follow_last: bool, budget: &mut Budget) -> Outcome {
    let mut followed = false;
    let mut missing: Option<usize> = None;
    while let Some((part, from_shell)) = todo.pop_front() {
        if part == ".." {
            if done
                .pop()
                .is_some_and(|p| p.contains(crate::cst::eval::TAGGED_PREFIX) || p == "safe-chains-unknown-workdir")
            {
                return unresolved();
            }
            if missing.is_some_and(|at| done.len() <= at) {
                missing = None;
            }
            continue;
        }
        if missing.is_some() || (todo.is_empty() && !follow_last) {
            done.push(part);
            continue;
        }
        let candidate = format!("{}/{part}", joined(&done).trim_end_matches('/'));
        // None: no match went through a link, so the literal component is still judged. A symlink
        // named `foo[bar]` does not match that pattern. When another match did go through a link,
        // the literal symlink is still what a quoted operand names, so it is judged too. An
        // unresolvable listing is a refusal, not a guess at the literal.
        if from_shell
            && part.contains(['*', '?', '['])
            && let Some(outcome) = glob(&done, &part, &todo, follow_last, budget)
        {
            let literal_link = matches!(lookup(&candidate), Lookup::Link(_));
            let unresolved = matches!(&outcome, Outcome::Followed(real) if real == crate::pathctx::UNRESOLVED_CWD);
            if !literal_link || unresolved {
                return outcome;
            }
        }
        match lookup(&candidate) {
            Lookup::Link(target) => {
                budget.links += 1;
                if budget.links > MAX_LINKS {
                    return unresolved();
                }
                let Some(target) = target else {
                    return unresolved();
                };
                if target.starts_with('/') {
                    done.clear();
                }
                for item in parts(&target, false).into_iter().rev() {
                    todo.push_front(item);
                }
                followed = true;
            }
            Lookup::Other => done.push(part),
            Lookup::Missing => {
                missing = Some(done.len());
                done.push(part);
            }
        }
    }
    if followed { Outcome::Followed(system_spelling(joined(&done))) } else { Outcome::Plain }
}

/// Each match of a glob component walked on its own. `None` when no match goes through a link, so
/// the pattern is judged as written; otherwise the one place every linked match reaches, or the
/// unresolvable placeholder when matches reach different places (some through a link, some not)
/// or cannot all be listed.
fn glob(done: &[String], pattern: &str, rest: &VecDeque<(String, bool)>, follow_last: bool, budget: &mut Budget) -> Option<Outcome> {
    let dir = joined(done);
    let entries = std::fs::read_dir(&dir).ok()?;
    let mut names = Vec::new();
    for entry in entries {
        budget.entries += 1;
        if budget.entries > MAX_ENTRIES {
            return Some(unresolved());
        }
        let Some(name) = entry.ok().and_then(|e| e.file_name().into_string().ok()) else {
            return Some(unresolved());
        };
        names.push(name);
    }
    let mut reached: Vec<String> = Vec::new();
    let mut plain = 0usize;
    for name in names {
        if name.starts_with('.') && !pattern.starts_with('.') {
            continue;
        }
        match crate::cst::opaque::glob::matches(pattern.as_bytes(), name.as_bytes()) {
            Some(true) => {}
            Some(false) => continue,
            None => return Some(unresolved()),
        }
        budget.matches += 1;
        if budget.matches > MAX_MATCHES {
            return Some(unresolved());
        }
        let mut next = rest.clone();
        next.push_front((name, false));
        match walk(done.to_vec(), next, follow_last, budget) {
            Outcome::Followed(real) if real == crate::pathctx::UNRESOLVED_CWD => return Some(unresolved()),
            Outcome::Followed(real) => reached.push(real),
            Outcome::Plain => plain += 1,
        }
    }
    reached.sort();
    reached.dedup();
    match reached.len() {
        0 => None,
        1 if plain == 0 => reached.pop().map(Outcome::Followed),
        _ if reached.iter().all(|real| stays_unprotected(real)) => None,
        _ => Some(unresolved()),
    }
}

/// Whether a place a glob match leads to lies in the project and is no more protected than the
/// project itself, so the matches can be judged together as the pattern is written. The place is
/// judged as a write, the stricter role.
fn stays_unprotected(real: &str) -> bool {
    let crate::pathctx::Root::Known(root) = crate::pathctx::root() else {
        return false;
    };
    if !root.starts_with('/') {
        return false;
    }
    let root = self::root(&root);
    let inside = real == root || real.starts_with(&format!("{}/", root.trim_end_matches('/')));
    inside && !real.contains(['*', '?', '[']) && crate::engine::resolve::write_target_verdict(real) != crate::verdict::Verdict::Denied
}

#[cfg(test)]
#[path = "physical_tests.rs"]
mod tests;
