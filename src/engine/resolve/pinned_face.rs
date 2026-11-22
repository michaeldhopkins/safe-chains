//! Reading a region role through the face an operation asks for.

use super::{Face, canonicalize, is_unpinnable};
use crate::engine::resolve::regions::{Role, classify_region, classify_text};
use crate::engine::facet::LocalLocus;

/// Classify an ALREADY-placed path: fail closed on an unpinnable spelling, then read the region
/// model's face, asked of both the place it lands and the way it was spelled.
pub(super) fn classify_pinned(placed: &crate::pathctx::Placed, want_write: Face) -> LocalLocus {
    if is_unpinnable(&canonicalize(placed.judged())) {
        return LocalLocus::Machine;
    }
    face_of(classify_region(placed), want_write)
}

/// The residue a substitution's tag leaves in a path: an ordinary relative stand-in, judged as text
/// because the place it names is the tag's.
pub(super) fn classify_residue(residue: &str, want_write: Face) -> LocalLocus {
    let canonical = canonicalize(residue);
    if is_unpinnable(&canonical) {
        return LocalLocus::Machine;
    }
    face_of(classify_text(&canonical), want_write)
}

fn face_of(role: Role, want_write: Face) -> LocalLocus {
    match want_write {
        Face::Read => role.read_locus,
        Face::Write => role.write_locus,
        Face::Rebind => role.rebind_locus,
    }
}
