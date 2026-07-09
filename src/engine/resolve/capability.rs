//! The capability builders: small constructors that each stamp out one `Capability`
//! (or fail-closed `Profile`) with the facet pairing its operation warrants. Resolvers
//! name the intent (`creates`/`overwrites`/`relocates`/`destroys`/`reads_*`/`worst`); the
//! enum choices and `because` strings live here, in one place.

use super::locus::classify_locus;
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
                reads_content(classify_locus(p), scale, "reads file content to the model")
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
    writes(
        Operation::Destroy,
        locus,
        scale,
        Reversibility::Effortful,        // recoverable only from backups (fail-closed)
        PersistenceLevel::Transient,     // a delete leaves nothing behind
        "rm deletes files (recoverable only from out-of-band backups)",
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

/// A moved-from source (`mv` src): `mutate` at `locus` — the entry leaves that directory —
/// `trivial` to undo (`mv` back), leaving nothing behind. NOT a destroy: the content
/// survives at the destination, which is why `mv` stays at write-local and `rm` does not.
pub(super) fn relocates(locus: LocalLocus, scale: Scale) -> Capability {
    writes(Operation::Mutate, locus, scale, Reversibility::Trivial, PersistenceLevel::Transient, "mv removes the source from its old location (trivially reversible: mv back)")
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
pub(super) fn transfer_profile(
    sources: &[&str],
    dest: &str,
    scale: Scale,
    per_source: impl Fn(LocalLocus, Scale) -> Capability,
    per_dest: impl Fn(LocalLocus, Scale) -> Capability,
) -> Profile {
    let mut caps: Vec<Capability> =
        sources.iter().map(|s| per_source(classify_locus(s), scale)).collect();
    caps.push(per_dest(classify_locus(dest), scale));
    Profile::of(caps)
}
