//! The unknown-folder mode of one evaluation: which folder level is in force, where each relative
//! path is placed, and which command leaves wrote without naming a path
//! (docs/design/unknown-folder-writes.md).
//!
//! Engaged only when a harness does not say which folder the command runs in, and the command
//! is classified at `targets::UNKNOWN_WORKDIR`. At `reads` nothing here changes a verdict. Above it:
//!
//! - a relative path is placed in the workspace when its anchor value and use are ones the level
//!   approves (`anchor::placement`), and left in the unknown folder otherwise, where no write lands
//!   inside anything;
//! - a command leaf whose verdict is a write is approved only when that write is accounted for: it
//!   wrote a path it named (and that path was placed or refused on its own), it declares what it
//!   writes in its folder (`writes_cwd`, `executor = "project"`), or a command nested in it was
//!   accounted for. Anything else is refused, so a writer nobody has labelled fails closed.

use std::cell::{Cell, RefCell};

use super::anchor::{self, Anchor, FolderLevel, Use};
use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

/// What the mode decided about one path or one command, for `--explain`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    Path { path: String, anchor: Anchor, use_: Use, placed: bool },
    Leaf { command: String, anchor: Option<Anchor>, admitted: bool },
}

#[derive(Default)]
struct Frame {
    named_write: bool,
    nested_accounted: bool,
    declared: Option<Anchor>,
}

thread_local! {
    static LEVEL: Cell<Option<FolderLevel>> = const { Cell::new(None) };
    static FRAMES: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
    static NOTES: RefCell<Vec<Note>> = const { RefCell::new(Vec::new()) };
}

/// Put `level` in force for the guard's lifetime and start a fresh record. Restored on drop.
#[must_use]
pub fn enter(level: FolderLevel) -> Guard {
    NOTES.with(|n| n.borrow_mut().clear());
    Guard(LEVEL.with(|l| l.replace(Some(level))))
}

pub struct Guard(Option<FolderLevel>);

impl Drop for Guard {
    fn drop(&mut self) {
        LEVEL.with(|l| l.set(self.0));
    }
}

/// The folder level in force, or `None` outside the mode.
pub fn level() -> Option<FolderLevel> {
    LEVEL.with(Cell::get)
}

/// Whether writes are being judged at all: a level above `reads` is in force.
pub fn judges_writes() -> bool {
    level().is_some_and(|l| l > FolderLevel::Reads)
}

/// What this evaluation decided, in order, for `--explain`.
pub fn notes() -> Vec<Note> {
    NOTES.with(|n| n.borrow().clone())
}

/// Whether `cwd` is the unknown folder or a directory below it (`cd sub` from there).
pub fn is_unknown(cwd: &str) -> bool {
    let base = crate::targets::UNKNOWN_WORKDIR;
    cwd.strip_prefix(base).is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// The workspace-relative path to classify `path` as, when it is relative to an unknown `cwd` and
/// the level places it; `None` leaves it in the unknown folder.
///
/// A path that climbs out of the folder, or cannot be read, is placed NOWHERE at every level, even
/// for a read: joined onto the unknown folder it would name an ordinary directory, while the real
/// parent of an unknown folder can be anything, another user's home included.
pub(super) fn place(cwd: &str, path: &str, stated: Option<Use>) -> Option<String> {
    let use_ = stated.unwrap_or(Use::Read);
    let level = level()?;
    if path.starts_with('/') || path.starts_with('~') || !is_unknown(cwd) {
        return None;
    }
    let below = cwd[crate::targets::UNKNOWN_WORKDIR.len()..].trim_start_matches('/');
    let relative = if below.is_empty() { path.to_string() } else { format!("{below}/{path}") };
    let anchor = anchor::of_path(&relative);
    let placed = anchor::placement(&relative, use_, level)
        .filter(|p| !use_.mutates() || ((p != "." || declares_writes_in_its_folder()) && !items_in_scope()));
    if level > FolderLevel::Reads && stated.is_some_and(Use::mutates) {
        record(Note::Path { path: relative.clone(), anchor, use_, placed: placed.is_some() });
    }
    match placed {
        None if anchor == Anchor::RelativeUnplaced || use_ == Use::Read => Some(NOWHERE.to_string()),
        other => other,
    }
}

/// Whether the command leaf being judged says what it writes in its folder (`writes_cwd` source or
/// output). Only such a tool may write to the folder ITSELF (`gofmt -w .`): a copy or a sync into
/// `.` writes names it brings with it (`cp /tmp/.zshrc .`), and the folder could be `~`.
fn declares_writes_in_its_folder() -> bool {
    FRAMES.with(|f| {
        f.borrow()
            .last()
            .and_then(|t| t.declared)
            .is_some_and(|a| matches!(a, Anchor::ImplicitSource | Anchor::ImplicitOutput))
    })
}

/// Whether a write may be naming an item that arrives at run time: an `xargs` item on stdin, or a
/// loop variable (`while read f`, `for f in …`). The classifier sees a stand-in for it, an ordinary
/// name, while the real item can be `.zshrc` (`ls -A | xargs -I{} sh -c 'echo x >> {}'`).
fn items_in_scope() -> bool {
    super::stdin_item_repr().is_some() || super::vars::any_loop()
}

/// The command leaf being judged wrote to a path it named.
pub(super) fn note_named_write() {
    FRAMES.with(|f| {
        if let Some(top) = f.borrow_mut().last_mut() {
            top.named_write = true;
        }
    });
}

/// A leaf classifier that judges each leaf it classifies, for a caller that hands one on.
pub(crate) fn judging(with_env: bool, classify: fn(&[Token]) -> Verdict) -> impl Fn(&[Token]) -> Verdict {
    move |tokens| judge_leaf(tokens, with_env, || classify(tokens))
}

/// Judge one command leaf: run `classify`, and when its verdict is a write made in the unknown
/// folder that nothing accounts for, refuse it.
pub(crate) fn judge_leaf(tokens: &[Token], with_env: bool, classify: impl FnOnce() -> Verdict) -> Verdict {
    // A relative path is classified as `bash` of that path. Account for it as that command, so the
    // shell's `writes_cwd = "code"` covers the project's own script. The classifier rewrites too.
    let rewritten = crate::handlers::direct_script::as_interpreter(tokens);
    let tokens = rewritten.as_ref();
    let declared = crate::registry::cwd_writes(tokens);
    let frame = FrameGuard::push(declared);
    let verdict = classify();
    let frame = frame.pop();
    let verdict = if verdict.is_allowed() { verdict.combine(folder_code_verdict(declared)) } else { verdict };
    let writes = matches!(verdict, Verdict::Allowed(l) if l > SafetyLevel::SafeRead);
    if !writes {
        return verdict;
    }
    let here_unknown = super::cwd().is_some_and(|c| is_unknown(&c));
    let Some(level) = level().filter(|l| *l > FolderLevel::Reads && here_unknown) else {
        let verdict = verdict.combine(cwd_write_verdict(declared, frame.named_write));
        if verdict.is_allowed() {
            mark_parent_accounted();
        }
        return verdict;
    };
    let names_its_writes = declared == Some(Anchor::NamesItsWrites) && frame.named_write;
    // An assignment in front can move a write the declaration describes (`GIT_INDEX_FILE=.zshrc git
    // add .`, `CARGO_TARGET_DIR=../.. cargo build`), so it voids an implicit declaration.
    let implicit = !with_env && declared.is_some_and(|a| level.admits_implicit(a));
    let accounted = implicit || names_its_writes || frame.nested_accounted;
    let command = tokens.iter().take(4).map(Token::as_str).collect::<Vec<_>>().join(" ");
    record(Note::Leaf { command, anchor: declared, admitted: accounted });
    if accounted {
        mark_parent_accounted();
        verdict
    } else {
        Verdict::Denied
    }
}

/// The verdict for the write a leaf makes in the folder it runs in without naming a path (`cargo
/// fmt`, `git commit`, `mise set`): that folder judged as a write target, so a `cd` earlier on the
/// line moves the write with it, exactly as it moves a relative operand. A leaf that declares a
/// fixed target (`writes_cwd = "none"`), or that named what it wrote and declares no folder write,
/// writes nothing here. The folder the harness reported is the session's own when it lies in the
/// project root, and is not judged, nor is an unknown one (the unknown-folder mode's), nor a missing
/// one, as for a relative path.
fn cwd_write_verdict(declared: Option<Anchor>, named_write: bool) -> Verdict {
    let in_folder = matches!(declared, Some(Anchor::ImplicitOutput | Anchor::ImplicitSource | Anchor::RunsFolderCode));
    let judged = super::cwd().is_some_and(|cwd| !is_unknown(&cwd) && (super::moved_by_cd() || !within_root(&cwd)));
    if declared == Some(Anchor::Free) || (named_write && !in_folder) || !judged {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let frame = FrameGuard::push(None);
    let verdict = crate::engine::resolve::write_target_verdict(".");
    frame.pop();
    verdict
}

/// The verdict for the code a leaf runs from the folder it runs in (`cargo run`, a `rake` task) or
/// builds there (`cargo build` runs build scripts): once a `cd` has moved the line, that folder is
/// judged as the place code is loaded from, so a temporary folder asks as `/tmp/x.py` would, though
/// writing there would be approved.
fn folder_code_verdict(declared: Option<Anchor>) -> Verdict {
    let runs = matches!(declared, Some(Anchor::ImplicitOutput | Anchor::RunsFolderCode));
    if !runs || super::cwd().is_none_or(|cwd| is_unknown(&cwd)) {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    if crate::engine::resolve::frozen_reach::is_unreviewed_folder(".") {
        return Verdict::Denied;
    }
    if !super::moved_by_cd() {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let frame = FrameGuard::push(None);
    let trusted = crate::engine::resolve::execute_file_verdict(".").is_allowed();
    frame.pop();
    if trusted { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }
}

fn within_root(cwd: &str) -> bool {
    super::root().is_some_and(|root| {
        let root = root.trim_end_matches('/');
        cwd.trim_end_matches('/') == root || cwd.starts_with(&format!("{root}/"))
    })
}

fn mark_parent_accounted() {
    FRAMES.with(|f| {
        if let Some(top) = f.borrow_mut().last_mut() {
            top.nested_accounted = true;
        }
    });
}

fn record(note: Note) {
    NOTES.with(|n| {
        let mut notes = n.borrow_mut();
        if notes.len() < MAX_NOTES && !notes.contains(&note) {
            notes.push(note);
        }
    });
}

/// What a path placed nowhere is classified as: a component the locus guard treats as unpinnable,
/// so a read of it is one the shield cannot clear and a write of it lands nowhere approvable.
const NOWHERE: &str = "__SAFE_CHAINS_CMDSUB__/outside-an-unknown-folder";

/// A long chain still explains its first writes; past this the record stops growing.
const MAX_NOTES: usize = 64;

/// Pops its frame even when the classification inside unwinds, so a later evaluation on the thread
/// does not inherit it.
struct FrameGuard(bool);

impl FrameGuard {
    fn push(declared: Option<Anchor>) -> FrameGuard {
        FRAMES.with(|f| f.borrow_mut().push(Frame { declared, ..Frame::default() }));
        FrameGuard(true)
    }

    fn pop(mut self) -> Frame {
        self.0 = false;
        FRAMES.with(|f| f.borrow_mut().pop()).unwrap_or_default()
    }
}

impl Drop for FrameGuard {
    fn drop(&mut self) {
        if self.0 {
            FRAMES.with(|f| f.borrow_mut().pop());
        }
    }
}

#[cfg(test)]
#[path = "folder_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "folder_soundness_tests.rs"]
mod soundness;
