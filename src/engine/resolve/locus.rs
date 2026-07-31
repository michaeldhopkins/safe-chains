//! Filesystem locus classification: which rung of the `LocalLocus` ladder a path argument
//! reaches (v1.4 §2.2). The path knowledge itself lives as DATA in `regions/default.toml`
//! (HP-20) — this module is the seam: it resolves the path against the harness cwd/root
//! (HP-19), applies the fail-closed `$VAR`/`..` guard (§0), then reads the region's role
//! through the operation-appropriate FACE.
//!
//! - `read_locus`  — the face a READ reaches. A recognized world-readable system path
//!   (`/etc/hosts`, `/proc/cpuinfo`) projects DOWN to a rung read-local admits, so the read
//!   passes; a secret store or unknown path stays at `machine`.
//! - `write_locus` — the face a WRITE reaches. System paths stay at `machine` (denied); it
//!   reproduces the pre-HP-20 single-locus behavior, so existing write call sites are
//!   unchanged. `classify_locus` is its alias (the conservative default face).

use std::borrow::Cow;

use super::regions::classify_region;
use crate::engine::facet::LocalLocus;

/// The locus a READ of `path` reaches (the read face of its region role).
pub(crate) fn read_locus(path: &str) -> LocalLocus {
    face(path, false)
}

/// The locus a WRITE of `path` reaches (the write face of its region role). The conservative
/// face — a system path stays at `machine` even where its read face is lower.
pub(crate) fn write_locus(path: &str) -> LocalLocus {
    face(path, true)
}

fn face(path: &str, want_write: bool) -> LocalLocus {
    // Scheme-aware: a URL is not an ordinary local path. `file:` names a LOCAL file, so classify
    // the path it points at (`file:///etc/shadow` denies like reading /etc/shadow). Any other
    // scheme (`http://`, `s3://`, `ssh://`, …) is a NETWORK endpoint — not a local filesystem
    // operation — so it admits here (the command's own handler gates the network) and a URL's
    // `..` is never misread as a filesystem escape. This is the one place the notion of a URL
    // lives; individual command handlers no longer special-case `file:`.
    if let Some(local) = file_url_local(path) {
        return classify_local(local, want_write);
    }
    if is_network_url(path) {
        // A URL consumed as network I/O admits at worktree — its handler gates the network, and a
        // URL's own `..` (`https://host/a/../b`) is a path segment, not a filesystem escape. But a
        // GENERIC read/write command (`cat`, `cp`, a redirect) treats `scheme://../../x` as a
        // LITERAL local path and the OS walks the `..`, climbing out of the workspace. So admit
        // only when the URL, read as a path, does NOT net-escape cwd (and carries no `$`/cmdsub).
        if path.contains('$') || path.contains("__SAFE_CHAINS_CMDSUB__") || url_escapes_cwd(path) {
            return LocalLocus::Machine;
        }
        return LocalLocus::Worktree;
    }
    classify_local(path, want_write)
}

/// Whether a scheme-URL string, read as a LOCAL filesystem path (how a generic reader like `cat`
/// treats it), would climb ABOVE cwd. The scheme label (`s3:`) and each normal segment are one
/// level down; each `..` one up. If depth ever goes negative, the `..`s escape the workspace
/// (`s3://../../x`) and it must not admit as a network endpoint. A real URL whose `..` stay
/// within their own path (`https://host/a/../b`) never goes negative — safe to admit.
fn url_escapes_cwd(url: &str) -> bool {
    let mut depth: i32 = 0;
    for seg in url.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                depth -= 1;
                if depth < 0 {
                    return true;
                }
            }
            _ => depth += 1,
        }
    }
    false
}

/// Default IFS. An UNQUOTED expansion whose value contains one of these is split by the shell into
/// several words, so one operand in the CST becomes several arguments at run time.
const IFS_WHITESPACE: [char; 3] = [' ', '\t', '\n'];

fn classify_local(path: &str, want_write: bool) -> LocalLocus {
    let whole = classify_one(path, want_write);

    // WORD SPLITTING. A variable holding whitespace expands to several words, and the classifier
    // saw one: `VAR="-rf /"; rm $VAR` was read as a single odd-looking operand and approved, while
    // the shell runs `rm -rf /`. `A="x /etc/shadow"; cat $A` read the shadow file the same way —
    // and note it turned on ORDER, since `"/etc/shadow x"` denied by starting at a hot region. So
    // each piece is classified in its own right and the worst wins.
    //
    // Combined with the unsplit answer rather than replacing it, so this can only ever tighten:
    // a value that happens to name a real file containing a space keeps whatever it classified as
    // before. Splitting terminates in one level — no piece contains whitespace.
    let expanded = crate::pathctx::expand_vars(path, want_write);
    if !expanded.contains(IFS_WHITESPACE) {
        return whole;
    }
    expanded
        .split(IFS_WHITESPACE)
        .filter(|piece| !piece.is_empty())
        .map(|piece| classify_one(piece, want_write))
        .fold(whole, LocalLocus::max)
}

fn classify_one(path: &str, want_write: bool) -> LocalLocus {
    // A bound `for`-loop variable expands to its list's representative item first (its read or
    // write representative), so `$f` inherits the list's locus; then the ambient cwd/root.
    let expanded = crate::pathctx::expand_vars(path, want_write);

    // An atom sentinel is `is_unpinnable` by default, so this is the ONLY thing that can neutralize
    // one — and it does so only where the atom is flanked by literal text in its own component.
    let expanded = Cow::Owned(neutralize_atoms(&expanded).into_owned());

    // A declared substitution's locus rides in its sentinel, and the sentinel can enter the path at
    // TWO points. Both are checked, and the worst wins along with the ordinary classification of
    // whatever surrounds it — the tag bounds where the substitution's own VALUE points and says
    // nothing about the text around it (`$(pwd)/.git/hooks/pre-commit`).
    //
    //  1. In the OPERAND, visible once variables are expanded. Checking before expansion missed a
    //     bound variable, which still spells the tag `$OUT` at this point.
    let (operand_tag, base) = match tagged_substitution(&expanded) {
        Some((tag, rewritten)) => (Some(tag), Cow::Owned(rewritten)),
        None => (None, expanded),
    };

    //  2. In the CWD, which only appears once `resolve` joins it on — so it is invisible above.
    //     Reachable since `cd $(…)` began carrying a locus instead of being silently ignored:
    //     `cd $(fd d /etc) && cat f` classified `f` as an ordinary relative name and approved a
    //     read under /etc. Same ordering mistake as (1), one layer further down.
    let resolved = crate::pathctx::resolve(&base);
    let (cwd_tag, base) = match tagged_substitution(&resolved) {
        Some((tag, rewritten)) => (Some(tag), Cow::Owned(rewritten)),
        None => (None, resolved),
    };

    let plain = classify_pinned(&base, want_write);
    [operand_tag, cwd_tag].into_iter().flatten().fold(plain, LocalLocus::max)
}

/// Classify an ALREADY-resolved path: canonicalize, fail closed on an unpinnable spelling, then
/// read the region model's face.
fn classify_pinned(resolved: &str, want_write: bool) -> LocalLocus {
    let canonical = canonicalize(resolved);
    if is_unpinnable(&canonical) {
        return LocalLocus::Machine;
    }
    let role = classify_region(&canonical);
    if want_write { role.write_locus } else { role.read_locus }
}

/// A path component standing in for a substitution's value while the SURROUNDING text is
/// classified. Deliberately an ordinary relative name, so it contributes nothing of its own and the
/// region model reads the residue exactly as it would in a literal path.
const SUB_STANDIN: &str = "sc_substitution_value";

/// Normalize path SPELLINGS that name the same file so the region model — chiefly the
/// exact-match config pin and the grant/shield lookups, which compare by string — can't be
/// dodged. Collapses `//` and `/.`-segments and rewrites an absolute `$HOME` prefix to `~`, so
/// `/Users/me/.config/safe-chains.toml`, `~/.config/./safe-chains.toml`, and `~/.config//…`
/// all reduce to the canonical `~/.config/safe-chains.toml`. `..` is left in place on purpose —
/// `is_unpinnable` rejects it (a normalized `..` would silently defeat that guard).
fn canonicalize(path: &str) -> Cow<'_, str> {
    let home = std::env::var("HOME").ok();
    let home_abs = home
        .as_deref()
        .filter(|h| !h.is_empty() && path.strip_prefix(*h).is_some_and(|r| r.is_empty() || r.starts_with('/')));
    let dotty = path.contains("//") || path.contains("/./") || path.ends_with("/.");
    if home_abs.is_none() && !dotty {
        return Cow::Borrowed(path);
    }
    let tilded = match home_abs {
        Some(h) if path.len() == h.len() => "~".to_string(),
        Some(h) => format!("~{}", &path[h.len()..]),
        None => path.to_string(),
    };
    if !(tilded.contains("//") || tilded.contains("/./") || tilded.ends_with("/.")) {
        return Cow::Owned(tilded);
    }
    let absolute = tilded.starts_with('/');
    let joined = tilded
        .split('/')
        .filter(|seg| !seg.is_empty() && *seg != ".")
        .collect::<Vec<_>>()
        .join("/");
    Cow::Owned(if absolute { format!("/{joined}") } else { joined })
}

/// The LOCAL path a `file:` URL names, or `None` when `path` is not a `file:` URL. Schemes are
/// case-insensitive; handles `file:///p`, `file://host/p`, and `file:/p`.
fn file_url_local(path: &str) -> Option<&str> {
    if path.len() < 5 || !path.as_bytes()[..5].eq_ignore_ascii_case(b"file:") {
        return None;
    }
    let rest = &path[5..];
    Some(rest.strip_prefix("//").map_or(rest, |authority| {
        authority.find('/').map_or("", |i| &authority[i..])
    }))
}

/// Whether `path` is a network URL: a `scheme://…` whose scheme is well-formed (a letter, then
/// letters / digits / `+` / `-` / `.`). A local path that merely contains `://` is not a URL.
fn is_network_url(path: &str) -> bool {
    let Some(idx) = path.find("://") else {
        return false;
    };
    let scheme = &path[..idx];
    scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
}

/// The default (write) face — kept as `classify_locus` so every existing write-side call site
/// reads unchanged.
pub(crate) fn classify_locus(path: &str) -> LocalLocus {
    write_locus(path)
}

/// Fail-closed guard (§0): a `$VAR` expansion, a `..` escape, or a COMMAND-substitution result
/// (`$(…)` / backticks, which the CST evaluates to the `__SAFE_CHAINS_CMDSUB__` placeholder)
/// could name ANYTHING, so no positive region classification is sound — worst-case to `machine`.
/// Without the substitution case, `rm $(echo /)` classifies the placeholder as a worktree path
/// and auto-approves `rm -rf /`. (Process substitution is a pipe whose inner command is checked
/// separately, so its distinct placeholder is NOT worst-cased here.)
pub(crate) fn is_unpinnable(path: &str) -> bool {
    path.contains('$')
        || path.contains("__SAFE_CHAINS_CMDSUB__")
        || path.contains(crate::cst::eval::ATOM_SENTINEL)
        // An atom sentinel that reached here UNNEUTRALIZED is one `neutralize_atoms` did not
        // confine, or a path that never went through it. Either way its value is unknown text, so
        // it worst-cases exactly like an opaque substitution. This is what keeps the atom claim
        // from widening anything by itself.
        
        || is_parent_escape(path)
}

/// The locus carried by a BOUNDED substitution sentinel, plus the path to classify in its place.
///
/// The sentinel usually appears as a path COMPONENT (`$(fd … app/)/lib`, `$(pwd)/.git/config`), so
/// the surrounding text matters as much as the tag: descending can reach a DIFFERENT rung than the
/// tag names, climbing can leave it entirely (`$(pwd)/../..`), and a second unpinnable piece
/// re-opens the hole the tag closed. The returned path has the sentinel replaced by a plain
/// component so the caller can run the ordinary region classification over the rest and take the
/// worse of the two.
fn tagged_substitution(path: &str) -> Option<(LocalLocus, String)> {
    use crate::engine::facet::FacetTerm;
    let prefix_len = crate::cst::eval::TAGGED_PREFIX.len();
    // EVERY tag in the word, not just the first. One word can hold several substitutions
    // (`$(pwd)/$(fd d /etc)`), and taking only the leading one classified the rest as ordinary
    // text — so a worktree tag in front hid a machine tag behind it.
    let mut worst: Option<LocalLocus> = None;
    let mut out = String::with_capacity(path.len());
    let mut rest = path;
    while let Some(at) = rest.find(crate::cst::eval::TAGGED_PREFIX) {
        let after = &rest[at + prefix_len..];
        // A token shaped like our sentinel that does not parse as one is not an ordinary filename
        // either — fail closed rather than let it through as text.
        let Some((term, tail)) = after.split_once("__") else {
            return Some((LocalLocus::Machine, String::new()));
        };
        let Some(locus) = LocalLocus::from_term(&term.to_lowercase().replace('_', "-")) else {
            return Some((LocalLocus::Machine, String::new()));
        };
        out.push_str(&rest[..at]);
        out.push_str(SUB_STANDIN);
        worst = Some(worst.map_or(locus, |w: LocalLocus| w.max(locus)));
        rest = tail;
    }
    let worst = worst?;
    out.push_str(rest);
    // Anything the region model cannot see through still fails closed here.
    if out.contains('$') || out.contains(UNPINNABLE_MARK) {
        return Some((LocalLocus::Machine, String::new()));
    }
    Some((worst, out))
}

const UNPINNABLE_MARK: &str = "__SAFE_CHAINS_CMDSUB__";

/// Turn a CONFINED atom sentinel into an ordinary path component, and worst-case every other one.
///
/// An atom claim is that the value is separator-free — so it cannot introduce a `/` and cannot
/// reach out of the component it sits in. That alone is not enough: a whole component that IS the
/// atom can still be `.` or `..`, which traverses without any separator of its own. Literal text
/// beside it in the same component rules that out, since `..` plus any other character is just a
/// filename. Both conditions are required, which is why this works per COMPONENT rather than over
/// the whole path.
///
/// The flanking must be LITERAL. Another sentinel next door is not evidence — two adjacent atoms
/// spell `..` between them (`$i$j` with `i=j="."`), and a locus tag says where its own value points,
/// not that the surrounding component is safe. So a component holding an atom plus any other
/// sentinel fails closed rather than counting the neighbour as flanking.
fn neutralize_atoms(path: &str) -> Cow<'_, str> {
    let atom = crate::cst::eval::ATOM_SENTINEL;
    if !path.contains(atom) {
        return Cow::Borrowed(path);
    }
    let out = path
        .split('/')
        .map(|comp| {
            if !comp.contains(atom) {
                return comp.to_string();
            }
            let residue = comp.replace(atom, "");
            // The flanking text must contain something that is not a DOT. Dots are the one
            // literal that does not make a component into a filename: `.` beside an atom whose
            // value is `.` spells `..`, and `..` beside an empty atom already IS `..`. Both
            // traverse with no separator anywhere, which is exactly what this was supposed to
            // rule out — `./out/.$(seq 1 1)` was admitted before this test existed.
            if residue.is_empty()
                || residue.chars().all(|c| c == '.')
                || residue.contains(crate::cst::eval::TAGGED_PREFIX)
            {
                return UNPINNABLE_MARK.to_string();
            }
            comp.replace(atom, SUB_STANDIN)
        })
        .collect::<Vec<_>>()
        .join("/");
    Cow::Owned(out)
}

/// Whether `path` carries a command-substitution sentinel of ANY kind — the opaque one
/// `is_unpinnable` worst-cases, or a bounded tagged one. (`TAGGED_PREFIX` is a prefix of the opaque
/// marker too, so one test covers both.)
///
/// Anything deciding "is this token an operand worth gating" must ask THIS, not a path-shape test
/// and not `is_unpinnable`. A sentinel carries no `/` and no `.`, so `looks_like_path` skips it;
/// keying on `is_unpinnable` used to cover that, but stopped once a declared substitution became
/// pinnable — which let `asciidoctor -o $(fd a /etc)` ship its write ungated.
pub(crate) fn is_substitution_value(path: &str) -> bool {
    path.contains(crate::cst::eval::TAGGED_PREFIX)
}

#[cfg(test)]
mod atom_backstop {
    /// The atom sentinel must be unpinnable ON ITS OWN.
    ///
    /// `neutralize_atoms` runs first in `classify_one` and rewrites every sentinel, so the locus
    /// path never consults this — which is exactly why it needs its own test. The callers that DO
    /// depend on it are the ones that ask `is_unpinnable` directly without neutralizing first
    /// (the pathgate's operand test, and the operands output claim); for them a sentinel read as an
    /// ordinary filename would be classified as a real path under whatever region it sits in.
    #[test]
    fn an_atom_sentinel_is_unpinnable_without_neutralization() {
        let atom = crate::cst::eval::ATOM_SENTINEL;
        assert!(super::is_unpinnable(atom), "a bare atom sentinel must fail closed");
        assert!(
            super::is_unpinnable(&format!("~/.ssh/dx_{atom}.txt")),
            "an un-neutralized atom must fail closed wherever it appears"
        );
        assert!(
            !super::is_unpinnable(&format!("~/.ssh/dx_{}.txt", super::SUB_STANDIN)),
            "the NEUTRALIZED form must be pinnable, or confinement could never pay off"
        );
    }
}

/// Whether the path's LITERAL structure names a credential store, whatever else in it is unknown.
///
/// The shield is a segment match on literal names, and in `cat ~/.ssh/$(id)` the `.ssh` is right
/// there in the command the user typed — interpolating a sibling component does not make it less of
/// a credential store. So this deliberately does NOT consult `is_unpinnable`.
///
/// That distinction matters if a secret FACET is ever built on top of the same region bit: a facet
/// makes a positive claim about what a command does, and claiming "reads a credential" for a value
/// nobody knows would be an invention, so a facet consumer wants the pinned question and should not
/// reuse this one. A pin-guarded variant existed for exactly that reason and was deleted here
/// rather than carried: its only caller was this nudge, and speculative dead code is how a
/// half-built rule rots.
pub(crate) fn names_credential_store(path: &str) -> bool {
    let expanded = crate::pathctx::expand_vars(path, false);
    let neutralized = neutralize_atoms(&expanded);
    classify_region(&crate::pathctx::resolve(&neutralized)).reads_secret
}

/// How firmly `path` is pinned — the `Anchoring` face of the same analysis the locus uses.
///
/// Reported rather than acted on: the locus already worst-cases an opaque path, so nothing here
/// decides a verdict. What it buys is an accurate REASON. An unconfined interpolation used to be
/// explained as "outside the working directory", which is both wrong and unactionable — `./out/$i`
/// is not outside anything, and the remedy is to flank the interpolation, not to grant a path.
pub(crate) fn anchoring_of(path: &str) -> crate::engine::facet::Anchoring {
    use crate::engine::facet::Anchoring;
    let neutralized = neutralize_atoms(path);
    if is_unpinnable(&neutralized) {
        Anchoring::Opaque
    } else if neutralized.as_ref() != path {
        Anchoring::Anchored
    } else {
        Anchoring::Literal
    }
}

fn is_parent_escape(path: &str) -> bool {
    path == ".." || path.starts_with("../") || path.contains("/../") || path.ends_with("/..")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::facet::LocalLocus;

    // The detailed path→role table is tested in `regions.rs` and end-to-end in the HP-20
    // scenario suite. Here we pin the SEAM: the fail-closed guard, the read/write asymmetry,
    // and that the write face reproduces the pre-HP-20 rungs.

    #[test]
    fn the_unpinnable_guard_worst_cases_both_faces() {
        for p in ["$HOME/.ssh/id_rsa", "$OUT/file", "../secret", "a/../../etc/passwd", "dir/.."] {
            assert_eq!(read_locus(p), LocalLocus::Machine, "read {p}");
            assert_eq!(write_locus(p), LocalLocus::Machine, "write {p}");
        }
    }

    #[test]
    fn write_face_admits_only_workspace_temp_and_streams() {
        assert_eq!(write_locus("/dev/null"), LocalLocus::Process);
        assert_eq!(write_locus("/tmp/scratch"), LocalLocus::Temp);
        assert_eq!(write_locus("notes.md"), LocalLocus::Worktree);
        assert_eq!(write_locus("src/engine/mod.rs"), LocalLocus::Worktree);
        assert_eq!(write_locus(".git/config"), LocalLocus::WorktreeTrusted, "in-project but write-frozen");
        // the retreat: everything outside the workspace denies — home, system, other users, devices
        assert_eq!(write_locus("~"), LocalLocus::Machine);
        assert_eq!(write_locus("~/notes"), LocalLocus::Machine);
        assert_eq!(write_locus("/etc/hosts"), LocalLocus::Machine);
        assert_eq!(write_locus("/usr/local/bin/x"), LocalLocus::Machine);
        assert_eq!(write_locus("/dev/rdisk0"), LocalLocus::Machine);
        assert_eq!(write_locus("~bob/.ssh/id_rsa"), LocalLocus::Machine, "another user's home");
    }

    #[test]
    fn read_face_admits_only_the_workspace_not_system_paths() {
        // the retreat: system/public paths are NO LONGER admitted — they deny (the harness then
        // prompts, or the user adds a read grant). We stopped modeling the filesystem.
        assert_eq!(read_locus("/etc/hosts"), LocalLocus::Machine);
        assert_eq!(read_locus("/usr/bin/python3"), LocalLocus::Machine);
        assert_eq!(read_locus("/etc/shadow"), LocalLocus::Machine);
        assert_eq!(read_locus("~/.ssh/id_rsa"), LocalLocus::Machine);
        assert_eq!(read_locus("~/notes"), LocalLocus::Machine);
        assert_eq!(read_locus("/some/unmapped/thing"), LocalLocus::Machine);
        // only the workspace and /tmp read
        assert_eq!(read_locus("notes.md"), LocalLocus::Worktree);
        assert_eq!(read_locus("/tmp/x"), LocalLocus::Temp);
    }

    #[test]
    fn file_urls_classify_the_local_path_they_name() {
        // every `file:` form, any case, resolves to the underlying local path
        for p in [
            "file:///etc/shadow",
            "file://localhost/etc/shadow",
            "file:/etc/shadow",
            "FILE:///etc/shadow",
            "File:///etc/shadow",
        ] {
            assert_eq!(read_locus(p), LocalLocus::Machine, "read {p}");
            assert_eq!(write_locus(p), LocalLocus::Machine, "write {p}");
        }
        // a file: URL classifies the local path it names — a system path denies both faces now
        assert_eq!(read_locus("file:///etc/hosts"), LocalLocus::Machine);
        assert_eq!(write_locus("file:///etc/hosts"), LocalLocus::Machine);
        // file: to a worktree-relative path stays worktree
        assert_eq!(read_locus("file:notes.txt"), LocalLocus::Worktree);
        // a `..` inside a file: URL is still a filesystem escape
        assert_eq!(read_locus("file://../../etc/shadow"), LocalLocus::Machine);
    }

    #[test]
    fn network_urls_are_not_local_operations() {
        // a network scheme admits (the network is the command handler's job), and a URL's `..`
        // is NOT a filesystem escape — so no over-deny.
        for p in ["http://example.com/a", "https://x/a/../b", "ftp://h/f", "s3://bucket/key", "ssh://h/p"] {
            assert_eq!(read_locus(p), LocalLocus::Worktree, "read {p}");
            assert_eq!(write_locus(p), LocalLocus::Worktree, "write {p}");
        }
        // a local path that merely contains `://` is NOT a URL
        assert_eq!(write_locus("/tmp/weird://name"), LocalLocus::Temp);
        assert_eq!(read_locus("./a:b"), LocalLocus::Worktree);
    }

    #[test]
    fn a_scheme_url_that_net_escapes_cwd_is_not_admitted() {
        // a real URL whose `..` stay within its own path still admits (no over-deny)
        assert_eq!(read_locus("https://x/a/../b"), LocalLocus::Worktree);
        assert_eq!(read_locus("s3://bucket/../key"), LocalLocus::Worktree);
        // but a `scheme://../../x` climbs above cwd when read as a local path → machine
        for p in ["s3://../../secret.txt", "gopher://../../etc/passwd", "s3://a/../../../etc/x"] {
            assert_eq!(read_locus(p), LocalLocus::Machine, "read {p}");
            assert_eq!(write_locus(p), LocalLocus::Machine, "write {p}");
        }
        // a `$` in a URL still worst-cases (an unpinnable value hiding as a URL)
        assert_eq!(read_locus("s3://$SECRET/x"), LocalLocus::Machine);
    }

    #[test]
    fn canonicalize_folds_equivalent_spellings() {
        // `//` and `/.` segments collapse so an exact-match region node can't be dodged
        assert_eq!(canonicalize("~/.config//safe-chains.toml"), "~/.config/safe-chains.toml");
        assert_eq!(canonicalize("~/.config/./safe-chains.toml"), "~/.config/safe-chains.toml");
        assert_eq!(canonicalize("/a//b/./c"), "/a/b/c");
        // `..` is left in place — the unpinnable guard rejects it (a folded `..` would defeat it)
        assert_eq!(canonicalize("~/a/../b"), "~/a/../b");
        // a clean path is returned untouched (borrowed)
        assert_eq!(canonicalize("~/.config/safe-chains.toml"), "~/.config/safe-chains.toml");
        // an absolute `$HOME` prefix rewrites to `~` so it hits the same node as the tilde form
        if let Some(home) = std::env::var("HOME").ok().filter(|h| h.starts_with('/')) {
            assert_eq!(canonicalize(&format!("{home}/.config/safe-chains.toml")), "~/.config/safe-chains.toml");
        }
    }

    #[test]
    fn credential_stores_are_named_as_such() {
        assert!(names_credential_store("~/.ssh/id_rsa"));
        assert!(names_credential_store("~/.aws/credentials"));
        assert!(names_credential_store("~/.gnupg/secring.gpg"));
        assert!(!names_credential_store("/etc/hosts")); // denied, but not a credential store
        assert!(!names_credential_store("notes.md"));
        // The reason this function exists rather than the pin-guarded one it replaced: a literal
        // shielded segment survives an interpolated sibling, in every spelling.
        assert!(names_credential_store("~/.ssh/__SAFE_CHAINS_CMDSUB__"));
        assert!(names_credential_store(&format!("~/.ssh/dx_{}.txt", crate::cst::eval::ATOM_SENTINEL)));
        assert!(!names_credential_store("~/projects/other/__SAFE_CHAINS_CMDSUB__"));
    }

    use proptest::prelude::*;
    proptest! {
        #[test]
        fn a_dollar_anywhere_forces_machine(s in ".{0,30}") {
            prop_assert_eq!(read_locus(&format!("{s}$")), LocalLocus::Machine);
            prop_assert_eq!(write_locus(&format!("{s}$")), LocalLocus::Machine);
        }

        #[test]
        fn a_parent_escape_forces_machine(s in "[a-zA-Z0-9/_]{0,20}") {
            prop_assert_eq!(write_locus(&format!("{s}/../x")), LocalLocus::Machine);
            prop_assert_eq!(write_locus(&format!("../{s}")), LocalLocus::Machine);
        }
    }
}
