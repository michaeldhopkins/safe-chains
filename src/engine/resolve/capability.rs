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

/// A `cp` source read: `observe` at the source locus, but with NO `local-process`
/// disclosure — the bytes are copied to a file, not printed to the model.
pub(super) fn copies_source(locus: LocalLocus, scale: Scale) -> Capability {
    let mut c = Capability::new(Operation::Observe);
    c.locus.local = locus;
    c.scale = scale;
    c.because = "cp reads the source file".to_string();
    c
}
