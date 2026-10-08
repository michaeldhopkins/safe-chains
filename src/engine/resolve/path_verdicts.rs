//! The verdicts for one path put to one use (a redirect's read or write, a tree read, a rebind, a
//! list-valued flag), out of `resolve.rs`.

use super::capability::{overwrites, reads_path};
use super::locus::{self, write_locus};
use crate::engine::facet::{Profile, Scale};

/// The verdict for READING the content of `path` — used to gate an input-redirect source
/// (`cmd < path`) by its read locus, exactly as an operand read is gated, so `cat < /etc/shadow`
/// denies like `cat /etc/shadow`. `-` / stdin never reaches here (redirects always name a file).
pub(crate) fn read_content_verdict(path: &str) -> crate::verdict::Verdict {
    let cap = reads_path(path, Scale::Single, "reads a redirect source");
    crate::engine::bridge::project(&Profile::of(vec![cap]))
}

/// The verdict for reading everything UNDER `path` — a recursive searcher or an archiver's source
/// tree, where the operand is the root of a sweep and not the file that gets read.
pub(crate) fn read_tree_verdict(path: &str) -> crate::verdict::Verdict {
    let cap = reads_path(path, Scale::Unbounded, "reads a tree of files");
    crate::engine::bridge::project(&Profile::of(vec![cap]))
}

/// The verdict for WRITING/overwriting `path` — used to gate a legacy writer command's file
/// operand (`tee`/`shred`/`bzip2`) by its write locus, so `shred /etc/hosts` denies.
pub(crate) fn write_target_verdict(path: &str) -> crate::verdict::Verdict {
    let cap = overwrites(write_locus(path), Scale::Single, false);
    crate::engine::bridge::project(&Profile::of(vec![cap]))
}

/// Whether REBINDING `path` (removing it, or pointing the name elsewhere) is refused where an
/// ordinary write is not. Only the trust-root directories answer true.
///
/// The nudge needs this, because its "does this reach outside" test reads the read and write faces
/// only. A grant on `~/.config` opens both, so `rm -rf ~/.config` looked entirely unremarkable to
/// the nudge while the engine refused it — a denial with no explanation at all, which is the worst
/// of the outcomes available.
pub(crate) fn rebind_is_stricter_than_write(path: &str) -> bool {
    let rebind = overwrites(locus::rebind_locus(path), Scale::Single, false);
    let refused = !crate::engine::bridge::project(&Profile::of(vec![rebind])).is_allowed();
    refused && write_target_verdict(path).is_allowed()
}

/// Judge a path value, taking the WORST element when it is a colon-separated LIST.
///
/// The one place the list rule lives, so the environment gate and the flag gate cannot drift apart.
/// They HAD drifted: the env gate always split on `:` while the flag gate never did, so
/// `BORG_RSH=x:/tmp/evil` denied and `borg --rsh x:/tmp/evil` — the same operation — was approved.
///
/// Splitting is opt-OUT rather than opt-in, because the two mistakes are not symmetric. Treating a
/// real list as one string is a FAIL-OPEN: `PYTHONPATH=/tmp/evil:/ok` read whole matches no locus
/// rule and sails through. Treating a single value as a list is merely stricter. So a value splits
/// unless its entry says it is a single value, and the entries that say so are commands
/// (`BORG_RSH`, `RSYNC_RSH`) rather than search paths.
///
/// A URL is never split: `https://example.com` is not `https` plus `//example.com`, and splitting
/// it denied every `curl` invocation in the suite.
pub(crate) fn worst_path_element(value: &str, judge: fn(&str) -> crate::verdict::Verdict, split_list: bool) -> crate::verdict::Verdict {
    let mut worst = judge(value);
    if split_list && value.contains(':') && !value.contains("://") {
        for element in value.split(':').filter(|s| !s.is_empty()) {
            worst = worst.combine(judge(element));
        }
    }
    worst
}

/// Running code a module directory supplies: `perl -I DIR -MMod` loads `DIR/Mod.pm` and runs it,
/// the same as running a script from DIR, so each directory is gated as that script would be.
pub(super) fn runs_modules_from(dirs: &[String]) -> impl Iterator<Item = super::Capability> + '_ {
    use super::capability::executes;
    use crate::engine::facet::ExecutionTrust;
    dirs.iter()
        .map(|dir| executes(super::classify_locus(dir), ExecutionTrust::CallerFile, "loads and runs modules from this directory"))
}
