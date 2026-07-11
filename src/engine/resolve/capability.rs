//! The capability builders: small constructors that each stamp out one `Capability`
//! (or fail-closed `Profile`) with the facet pairing its operation warrants. Resolvers
//! name the intent (`creates`/`overwrites`/`relocates`/`destroys`/`reads_*`/`worst`); the
//! enum choices and `because` strings live here, in one place.

use super::locus::{classify_locus, read_locus, write_locus};
use crate::engine::facet::*;

/// One `observe · content-to-model` capability per path (empty list = reads stdin). A
/// `-` operand is stdin (process-scoped); every other path is placed by `classify_locus`.
pub(super) fn reads_to_model(paths: &[&str], scale: Scale) -> Vec<Capability> {
    if paths.is_empty() {
        return vec![reads_content(LocalLocus::Process, scale, "reads stdin")];
    }
    paths
        .iter()
        .map(|p| {
            if *p == "-" {
                reads_content(LocalLocus::Process, scale, "reads stdin (-)")
            } else {
                reads_content(read_locus(p), scale, "reads file content to the model")
            }
        })
        .collect()
}

pub(super) fn reads_content(locus: LocalLocus, scale: Scale, because: &str) -> Capability {
    let mut c = Capability::new(Operation::Observe);
    c.locus.local = locus;
    c.scale = scale;
    c.disclosure.audience = DisclosureAudience::LocalProcess; // content → the model
    c.because = because.to_string();
    c
}

pub(super) fn destroys(locus: LocalLocus, scale: Scale) -> Capability {
    // Worktree/temp data is recoverable with effort (VCS, reinstall, regenerate). But an
    // UNBOUNDED destroy reaching home or the system (locus >= user) has no such recovery path
    // — `rm -rf /`, `rm -rf ~` wipe irreplaceable data — so it worst-cases to irreversible
    // (HP-8). That `destroy · irreversible · unbounded` signature is the one corner even yolo
    // refuses; a single or bounded system delete (rm /etc/hosts) stays effortful.
    let reversibility = if locus >= LocalLocus::User && scale == Scale::Unbounded {
        Reversibility::Irreversible
    } else {
        Reversibility::Effortful
    };
    writes(
        Operation::Destroy,
        locus,
        scale,
        reversibility,
        PersistenceLevel::Transient, // a delete leaves nothing behind
        "rm deletes files (recoverable only from out-of-band backups; irreversible when it mass-deletes home/system)",
    )
}

/// The private builder behind the write-family capability constructors (`creates`,
/// `overwrites`, `relocates`, `destroys`): a write at `locus` with the reversibility and
/// persistence the operation warrants. Resolvers call the named constructors, never this —
/// the intent (and the enum pairing) then lives in exactly one place.
fn writes(
    op: Operation,
    locus: LocalLocus,
    scale: Scale,
    reversibility: Reversibility,
    persistence: PersistenceLevel,
    because: &str,
) -> Capability {
    let mut c = Capability::new(op);
    c.locus.local = locus;
    c.scale = scale;
    c.reversibility = reversibility;
    c.persistence.level = persistence;
    c.because = because.to_string();
    c
}

/// A fresh file or directory (`mkdir`, `touch`): `create` at `locus`, `trivial` to undo
/// (`rmdir`/`rm` the new entry), leaving ordinary data.
pub(super) fn creates(locus: LocalLocus, scale: Scale) -> Capability {
    writes(Operation::Create, locus, scale, Reversibility::Trivial, PersistenceLevel::Data, "creates a file or directory")
}

/// A destination write that may clobber existing content (`cp`/`mv` dest): `create` at
/// `locus`, `recoverable` (the repo-recoverable assumption, HP-8) — or `trivial` when
/// `--no-clobber` guarantees no overwrite.
pub(super) fn overwrites(locus: LocalLocus, scale: Scale, no_clobber: bool) -> Capability {
    let reversibility = if no_clobber { Reversibility::Trivial } else { Reversibility::Recoverable };
    writes(Operation::Create, locus, scale, reversibility, PersistenceLevel::Data, "writes the destination; may overwrite existing content unless --no-clobber")
}

/// A dump/export command's OUTPUT FILE (`supabase db dump -f`, `pg_dump --file`): `create` at
/// `locus`, `recoverable` (it may clobber an existing file; worktree content is repo-recoverable,
/// HP-8), leaving data. `single` scale (one file), gated at the file's locus exactly like a
/// redirect — `-f ./out.sql` is a worktree write, `-f /etc/cron.d/job` a system write. The bulk
/// REMOTE read is a separate capability (the `data-export` archetype); this is only the local sink.
pub(super) fn writes_export_file(locus: LocalLocus) -> Capability {
    writes(Operation::Create, locus, Scale::Single, Reversibility::Recoverable, PersistenceLevel::Data, "writes the export/dump output file (may overwrite existing content)")
}

/// An in-place edit of an existing file (`sed -i`): `mutate` at `locus`, `recoverable` (the
/// old content is gone unless a backup was kept, but worktree content is repo-recoverable,
/// HP-8), leaving data. Distinct from `overwrites` (a fresh dest) — the file is edited, not
/// replaced wholesale.
pub(super) fn mutates(locus: LocalLocus, scale: Scale, because: &str) -> Capability {
    writes(Operation::Mutate, locus, scale, Reversibility::Recoverable, PersistenceLevel::Data, because)
}

/// A moved-from source (`mv` src): `mutate` at `locus` — the entry leaves that directory —
/// `trivial` to undo (`mv` back), leaving nothing behind. NOT a destroy: the content
/// survives at the destination, which is why `mv` stays at write-local and `rm` does not.
pub(super) fn relocates(locus: LocalLocus, scale: Scale) -> Capability {
    writes(Operation::Mutate, locus, scale, Reversibility::Trivial, PersistenceLevel::Transient, "mv removes the source from its old location (trivially reversible: mv back)")
}

/// Running code: `execute` at the EXECUTOR's `locus`, with the supplied `trust` (`SelfCode`
/// for the project's own build artifact, `CallerFile` for a named script file). The code's
/// downstream effects are deliberately NOT modeled here — bounding them is the sandbox's job
/// (`Isolation`), not a static string classifier's. This capability is the act of invoking an
/// executor, gated by WHERE that executor lives: a worktree-local one is the dev loop (the
/// `developer` level admits it), a foreign one (`/tmp`, `~`, `/usr/local/bin`) or an
/// unpinnable path (`$VAR`/glob → `machine`) denies on locus. Modest facets (no forced
/// worst-case) so a worktree executor projects to `developer`. See
/// docs/design/behavioral-taxonomy-execution-origin.md.
pub(super) fn executes(locus: LocalLocus, trust: ExecutionTrust, because: &str) -> Capability {
    let mut c = Capability::new(Operation::Execute);
    c.locus.local = locus;
    c.execution.trust = trust;
    c.because = because.to_string();
    c
}

/// The fail-closed profile (§0): a single worst-case capability citing `because` — the
/// standard return when a resolver cannot certify an invocation (unknown flag, missing
/// operand, spoofed path).
pub(super) fn worst(because: &str) -> Profile {
    Profile::of(vec![Capability::worst(because)])
}

/// Breadth of a filesystem effect: `unbounded` when recursing, `bounded` for a glob or
/// several operands, else `single`. Shared by rm/mkdir/touch/cp.
pub(super) fn breadth_scale(operands: &[&str], recursive: bool) -> Scale {
    if recursive {
        Scale::Unbounded
    } else if operands.len() > 1 || operands.iter().any(|p| p.contains(['*', '?', '['])) {
        Scale::Bounded
    } else {
        Scale::Single
    }
}

/// A non-disclosing read: `observe` at `locus` with NO `local-process` disclosure — the
/// content flows to a file or link, not to the model. Used by `cp` (its source) and `ln`
/// (its target, whose content becomes reachable *through* the link — cp-by-reference), so
/// a home/system operand denies on the read locus just as it would for `cat`.
pub(super) fn observes(locus: LocalLocus, scale: Scale, because: &str) -> Capability {
    let mut c = Capability::new(Operation::Observe);
    c.locus.local = locus;
    c.scale = scale;
    c.because = because.to_string();
    c
}

/// The profile of a content-transfer command (`cp`/`mv`/`ln`): one capability per SOURCE
/// operand plus one for the DEST, each gated at its own locus. Assembling it here — rather
/// than by hand in each resolver — makes a *dropped operand role* unrepresentable: every
/// source flows through `per_source` and the dest through `per_dest`, by construction, so
/// the omission that made `ln` a `cp`-bypass (HP-18) cannot recur. Callers close over any
/// extra parameters (the `because` string, the `--no-clobber` flag) to fit the uniform
/// `Fn(locus, scale) -> Capability` shape.
///
/// `source_writes` selects the source locus FACE: `mv` REMOVES its source (a write — the entry
/// leaves that directory), so its source must gate at `write_locus`, not `read_locus`. `cp`/`ln`
/// only READ their source. The two faces diverge for roles where read and write policy differ
/// (a copy-OK-but-don't-delete location), so gating a relocate at the read face would be a
/// fail-open there; this closes it by construction rather than relying on the locus ladder.
pub(super) fn transfer_profile(
    sources: &[&str],
    dest: &str,
    scale: Scale,
    source_writes: bool,
    per_source: impl Fn(LocalLocus, Scale) -> Capability,
    per_dest: impl Fn(LocalLocus, Scale) -> Capability,
) -> Profile {
    let source_locus = if source_writes { write_locus } else { read_locus };
    let mut caps: Vec<Capability> =
        sources.iter().map(|s| per_source(source_locus(s), scale)).collect();
    caps.push(per_dest(classify_locus(dest), scale));
    Profile::of(caps)
}
