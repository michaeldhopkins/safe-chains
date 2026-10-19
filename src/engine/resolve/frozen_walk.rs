//! Walking the tree below a recursive operand for the write-frozen names it reaches.
//!
//! Without `follow`, links below the operand are not entered (`chmod -R`, `cp -r`, `find`). With it
//! (`find -L`, `fd -L`, `cp -RL`, `rsync -L`), a link to a folder inside the project is entered once,
//! and a link out of the tree is reported under its own name so the caller judges where it leads.
//! A traversal's own filters (`find -path ./.git -prune`) narrow what is reached; see [`Skip`].

use std::collections::{HashSet, VecDeque};

use super::frozen_reach::{Reach, Sight, fold_sight, sight_of, stops_before_matches};

pub(crate) const MAX_ENTRIES: usize = 20_000;
const MAX_LINKS: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum WalkLinks {
    Ignore,
    Items,
    Follow,
}

/// How a recursive command walks: whether it lists hidden entries, how it treats links, and which
/// entries its own filters keep from its action.
#[derive(Clone, Copy)]
pub(crate) struct Walk<'a> {
    pub hidden: bool,
    pub links: WalkLinks,
    pub skip: &'a Skip,
}

impl Walk<'_> {
    pub(crate) fn plain(hidden: bool) -> Walk<'static> {
        Walk { hidden, links: WalkLinks::Ignore, skip: &NO_SKIP }
    }

    pub(crate) fn follow() -> Walk<'static> {
        Walk { hidden: true, links: WalkLinks::Follow, skip: &NO_SKIP }
    }

    pub(crate) fn items(hidden: bool, follow: bool) -> Walk<'static> {
        Walk { hidden, links: if follow { WalkLinks::Follow } else { WalkLinks::Items }, skip: &NO_SKIP }
    }
}

static NO_SKIP: Skip = Skip { pruned: Vec::new(), excluded: Vec::new(), files_only: false };

/// The entries a traversal's filters keep from its action: `pruned` ones are neither acted on nor
/// entered (`-path ./.git -prune -o …`), `excluded` ones are not acted on but are entered
/// (`-not -path './.git/*'`), and `files_only` keeps folders from the action (`-type f`). A test
/// whose pattern cannot be read matches nothing, so it keeps nothing away.
#[derive(Default, Debug, PartialEq, Eq)]
pub(crate) struct Skip {
    pub pruned: Vec<(Test, bool)>,
    pub excluded: Vec<Test>,
    pub files_only: bool,
}

/// One `find` test: `-path`/`-wholename` against the path as find prints it, `-name` against the
/// last component, each optionally without case. The `bool` beside a pruned test is `-type d`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Test {
    Path(String, bool),
    Name(String, bool),
}

impl Test {
    fn holds(&self, path: &str, name: &str) -> bool {
        let (pattern, against, fold) = match self {
            Test::Path(p, fold) => (p, path, *fold),
            Test::Name(p, fold) => (p, name, *fold),
        };
        let (pattern, against) =
            if fold { (pattern.to_lowercase(), against.to_lowercase()) } else { (pattern.clone(), against.to_string()) };
        crate::cst::opaque::glob::matches(pattern.as_bytes(), against.as_bytes()) == Some(true)
    }

    /// Whether every entry below the folder at `path` holds: a path pattern `X/*` whose `X` matches
    /// the folder, since `*` also matches `/`.
    fn holds_below(&self, path: &str) -> bool {
        matches!(self, Test::Path(p, fold) if p.strip_suffix("/*").is_some_and(|x| Test::Path(x.to_string(), *fold).holds(path, "")))
    }
}

impl Skip {
    fn pruned(&self, path: &str, name: &str, dir: bool) -> bool {
        self.pruned.iter().any(|(t, dir_only)| (dir || !dir_only) && t.holds(path, name))
    }

    fn excluded(&self, path: &str, name: &str, dir: bool) -> bool {
        (dir && self.files_only) || self.excluded.iter().any(|t| t.holds(path, name))
    }

    fn excludes_below(&self, path: &str) -> bool {
        self.excluded.iter().any(|t| t.holds_below(path))
    }
}

pub(crate) fn unlisted(base: &str) -> Reach {
    Reach::Unlisted(format!("{}/.git/f", base.trim_end_matches('/')))
}

/// The frozen names, and with `follow` the links out of the tree, below the operand `base`. Empty
/// when nothing is reached, when `base` cannot be placed on disk, or when `base` already lies
/// outside the readable and writable sandbox. Without `follow` the walk stops at the first frozen
/// name. A spelling that contains `*`, `?` or `[` is the literal path, when it exists, and every match.
pub(crate) fn reach_below(base: &str, walk: Walk) -> Vec<Reach> {
    let mut seen = 0;
    reach_below_with(base, walk, &mut seen)
}

fn reach_below_with(base: &str, walk: Walk, seen: &mut usize) -> Vec<Reach> {
    if !base.contains(['*', '?', '[']) {
        return walk_literal(base, walk, seen);
    }
    // Literal first, so a clean match cannot hide a frozen name there. `outside_sandbox` globs `*` and `[`.
    let spelled = spelled_dir(base).filter(|dir| in_project(dir) || !outside_sandbox(base));
    let mut reached = spelled.as_ref().map(|dir| walk_placed(base, dir, walk, seen)).unwrap_or_default();
    if stops_before_matches(walk.links == WalkLinks::Ignore, sight_of(&reached)) {
        return reached;
    }
    reached.extend(through_glob(base, walk, seen, spelled.as_deref()));
    reached
}

/// The operand joined onto the working directory when that path exists. Lexical, so `*` and `[` stay literal.
fn spelled_dir(base: &str) -> Option<String> {
    if base.contains('$') || base.contains(crate::cst::eval::TAGGED_PREFIX) {
        return None;
    }
    let cwd = crate::pathctx::cwd().filter(|c| c.starts_with('/'))?;
    let trimmed = base.trim_end_matches('/');
    let path = if base.starts_with('/') {
        if trimmed.is_empty() { "/".to_string() } else { trimmed.to_string() }
    } else if trimmed.is_empty() || trimmed == "." {
        cwd
    } else {
        format!("{}/{trimmed}", cwd.trim_end_matches('/'))
    };
    std::fs::symlink_metadata(&path).is_ok().then_some(path)
}

fn in_project(path: &str) -> bool {
    let Some(root) = crate::pathctx::root().filter(|r| r.starts_with('/')) else { return false };
    let root = root.trim_end_matches('/');
    let path = path.trim_end_matches('/');
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.into())
        .starts_with(std::fs::canonicalize(root).unwrap_or_else(|_| root.into()))
}

/// The concrete directory `base`, placed through the ordinary path resolver.
fn walk_literal(base: &str, walk: Walk, seen: &mut usize) -> Vec<Reach> {
    let Some(dir) = on_disk(base) else {
        let unplaced = base.contains('$') || base.contains(crate::cst::eval::TAGGED_PREFIX);
        return if unplaced && crate::pathctx::root().is_some_and(|r| r.starts_with('/')) { vec![unlisted(base)] } else { Vec::new() };
    };
    if outside_sandbox(base) {
        return Vec::new();
    }
    walk_placed(base, &dir, walk, seen)
}

/// `dir` is the directory already listed for the operand spelling `base`.
fn walk_placed(base: &str, dir: &str, walk: Walk, seen: &mut usize) -> Vec<Reach> {
    if walk.links != WalkLinks::Follow && !base.ends_with('/') && std::fs::symlink_metadata(dir).is_ok_and(|m| m.file_type().is_symlink()) {
        return Vec::new();
    }
    // A filter is matched against the path as find prints it, which a trailing `/` changes.
    let skip = if base.len() > 1 && base.ends_with('/') { &NO_SKIP } else { walk.skip };
    Walker { base: base.trim_end_matches('/'), dir, walk: Walk { skip, ..walk }, reached: Vec::new(), links: 0 }.run(seen)
}

struct Walker<'a> {
    base: &'a str,
    dir: &'a str,
    walk: Walk<'a>,
    reached: Vec<Reach>,
    links: usize,
}

impl Walker<'_> {
    fn run(mut self, seen: &mut usize) -> Vec<Reach> {
        let root = (self.walk.links != WalkLinks::Ignore)
            .then(crate::pathctx::root)
            .flatten()
            .and_then(|r| std::fs::canonicalize(r).ok());
        let walked = std::fs::canonicalize(self.dir).unwrap_or_else(|_| self.dir.into());
        let mut entered = HashSet::new();
        let mut missed = false;
        let mut queue = VecDeque::from([String::new()]);
        while let Some(rel) = queue.pop_front() {
            let here = if rel.is_empty() { self.dir.to_string() } else { format!("{}/{rel}", self.dir) };
            let entries = match open_dir(&here) {
                Opened::Entries(entries) => entries,
                Opened::Unlisted => {
                    missed = true;
                    continue;
                }
                Opened::Absent => continue,
            };
            for entry in entries {
                *seen += 1;
                let Some(entry) = entry.ok().filter(|_| *seen <= MAX_ENTRIES) else { return vec![unlisted(self.base)] };
                let Ok(name) = entry.file_name().into_string() else { return vec![unlisted(self.base)] };
                if !self.walk.hidden && name.starts_with('.') {
                    continue;
                }
                let below = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
                let shown = format!("{}/{below}", self.base);
                let kind = entry.file_type().ok();
                let is_link = kind.is_some_and(|t| t.is_symlink());
                let is_dir =
                    if is_link { std::fs::metadata(entry.path()).is_ok_and(|m| m.is_dir()) } else { kind.is_some_and(|t| t.is_dir()) };
                if self.walk.skip.pruned(&shown, &name, is_dir && (!is_link || self.walk.links == WalkLinks::Follow)) {
                    continue;
                }
                if is_frozen(&name) {
                    self.frozen(&shown, &name, is_dir);
                    if self.walk.links == WalkLinks::Ignore && !self.reached.is_empty() {
                        return self.reached;
                    }
                    continue;
                }
                if !is_link {
                    if is_dir {
                        queue.push_back(below);
                    }
                    continue;
                }
                if self.walk.links != WalkLinks::Follow {
                    if self.walk.links == WalkLinks::Items
                        && !self.walk.skip.files_only
                        && !self.walk.skip.excluded(&shown, &name, false)
                        && !std::fs::canonicalize(entry.path()).is_ok_and(|t| clean_in_root(&t, root.as_deref()))
                        && !self.push_link(shown)
                    {
                        return vec![Reach::Unlisted(super::locus::UNKNOWABLE_ITEM.to_string())];
                    }
                    continue;
                }
                match std::fs::canonicalize(entry.path()) {
                    Ok(target) if is_dir && clean_in_root(&target, root.as_deref()) => {
                        if entered.insert(target) {
                            queue.push_back(below);
                        }
                    }
                    Ok(target) if !is_dir && target.starts_with(&walked) && clean_in_root(&target, root.as_deref()) => {}
                    _ => {
                        if !self.push_link(if is_dir { format!("{shown}/.git/f") } else { shown }) {
                            return vec![Reach::Unlisted(super::locus::UNKNOWABLE_ITEM.to_string())];
                        }
                    }
                }
            }
        }
        let seen = fold_sight(sight_of(&self.reached), if missed { Sight::Unlisted } else { Sight::Empty });
        if seen == Sight::Unlisted { vec![unlisted(self.base)] } else { self.reached }
    }

    fn push_link(&mut self, item: String) -> bool {
        self.links += 1;
        if self.links > MAX_LINKS {
            return false;
        }
        self.reached.push(Reach::Found(item));
        true
    }

    /// A frozen entry is reached through what lies inside it unless the filters keep all of that
    /// away, and then as itself unless they keep it away too.
    fn frozen(&mut self, shown: &str, name: &str, is_dir: bool) {
        if !(is_dir && self.walk.skip.excludes_below(shown)) {
            self.reached.push(Reach::Found(format!("{shown}/f")));
        } else if !self.walk.skip.excluded(shown, name, is_dir) {
            self.reached.push(Reach::Found(shown.to_string()));
        }
    }
}

/// Each match of a glob operand, as a literal name (`*` in a match must not be re-globbed). `walked`
/// was already visited. A pattern above the last component is unlisted.
fn through_glob(base: &str, walk: Walk, seen: &mut usize, walked: Option<&str>) -> Vec<Reach> {
    let walk = Walk { skip: &NO_SKIP, ..walk };
    let trimmed = base.trim_end_matches('/');
    let (parent, pattern) = trimmed.rsplit_once('/').map_or((".", trimmed), |(p, l)| (if p.is_empty() { "/" } else { p }, l));
    if parent.contains(['*', '?', '[']) {
        return on_disk(".").map(|_| unlisted(base)).into_iter().collect();
    }
    if outside_sandbox(parent) {
        return Vec::new();
    }
    let Some(parent_path) = on_disk(parent) else { return Vec::new() };
    let entries = match open_dir(&parent_path) {
        Opened::Entries(entries) => entries,
        Opened::Unlisted => return vec![unlisted(base)],
        Opened::Absent => return Vec::new(),
    };
    let mut reached = Vec::new();
    for entry in entries {
        *seen += 1;
        let Some(entry) = entry.ok().filter(|_| *seen <= MAX_ENTRIES) else { return vec![unlisted(base)] };
        // Keep the listed path. Resolving the name again treats `*`, `?` or `[` as a pattern.
        let listed = entry.path();
        let Some(name) = entry.file_name().into_string().ok() else { return vec![unlisted(base)] };
        if name.starts_with('.') && !pattern.starts_with('.') {
            continue;
        }
        match crate::cst::opaque::glob::matches(pattern.as_bytes(), name.as_bytes()) {
            Some(true) => {}
            Some(false) => continue,
            None => return vec![unlisted(base)],
        }
        if walked.is_some_and(|dir| same_place(dir, &listed)) {
            continue;
        }
        let frozen = is_frozen(&name);
        let each = if parent == "." { name } else { format!("{}/{name}", parent.trim_end_matches('/')) };
        if frozen {
            reached.push(Reach::Found(format!("{each}/f")));
        } else if let Some(dir) = listed.to_str().filter(|_| each.contains(['*', '?', '['])) {
            reached.extend(walk_placed(&each, dir, walk, seen));
        } else {
            reached.extend(walk_literal(&each, walk, seen));
        }
        if stops_before_matches(walk.links == WalkLinks::Ignore, sight_of(&reached)) {
            reached.retain(|r| matches!(r, Reach::Found(_)));
            reached.truncate(1);
            return reached;
        }
    }
    reached
}

enum Opened {
    Entries(std::fs::ReadDir),
    Unlisted,
    Absent,
}

/// [`Opened::Unlisted`] when a directory cannot be listed. [`Opened::Absent`] for a missing path or a file.
fn open_dir(path: &str) -> Opened {
    match std::fs::read_dir(path) {
        Ok(entries) => Opened::Entries(entries),
        Err(err) if dir_unreadable(&err, path) => Opened::Unlisted,
        Err(_) => Opened::Absent,
    }
}

fn dir_unreadable(err: &std::io::Error, path: &str) -> bool {
    !matches!(err.kind(), std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory)
        && match std::fs::metadata(path) {
            Ok(meta) => meta.is_dir(),
            Err(stat_err) => !matches!(stat_err.kind(), std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory),
        }
}

fn same_place(walked: &str, listed: &std::path::Path) -> bool {
    listed.to_str() == Some(walked)
        || std::fs::canonicalize(walked)
            .ok()
            .zip(std::fs::canonicalize(listed).ok())
            .is_some_and(|(a, b)| a == b)
}

fn outside_sandbox(base: &str) -> bool {
    let repr = format!("{}/f", base.trim_end_matches('/'));
    let placed = crate::pathctx::on_the_link(|| crate::pathctx::resolve(&repr));
    let canonical = super::locus::canonicalize(&placed);
    (placed.starts_with('/') || placed.starts_with('~'))
        && (super::locus::is_unpinnable(&canonical) || {
            let role = super::regions::classify_region(&canonical);
            role.read_locus >= crate::engine::facet::LocalLocus::User && role.write_locus >= crate::engine::facet::LocalLocus::User
        })
}

fn clean_in_root(target: &std::path::Path, root: Option<&std::path::Path>) -> bool {
    root.and_then(|r| target.strip_prefix(r).ok())
        .is_some_and(|rel| rel.components().all(|c| c.as_os_str().to_str().is_some_and(|s| !is_frozen(s))))
}

pub(crate) fn is_frozen(name: &str) -> bool {
    let fold = cfg!(target_os = "macos");
    super::regions::metadata::frozen_names()
        .iter()
        .any(|n| if fold { n.eq_ignore_ascii_case(name) } else { *n == name })
}

pub(crate) fn on_disk(base: &str) -> Option<String> {
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
#[path = "frozen_walk_tests.rs"]
mod tests;
