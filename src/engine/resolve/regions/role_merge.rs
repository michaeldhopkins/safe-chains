//! How two readings of one path merge into the worse one, and which faces a grant may widen.

use super::{Frozen, Role};
use crate::engine::facet::LocalLocus;

impl Role {
    /// Only a NAME rule the other reading trips counts: a link to a temporary folder is foreign
    /// code however the project spells it, so its lower locus is not lifted to the spelling's.
    pub(super) fn worse(self, other: Role) -> Role {
        let rank = |frozen: Frozen| match frozen {
            Frozen::Nothing => 0,
            Frozen::Rebind => 1,
            Frozen::Write => 2,
        };
        if rank(other.frozen) <= rank(self.frozen) && (self.reads_secret || !other.reads_secret) {
            return self;
        }
        Role {
            read_locus: self.read_locus.max(other.read_locus),
            write_locus: self.write_locus.max(other.write_locus),
            rebind_locus: self.rebind_locus.max(other.rebind_locus),
            reads_secret: self.reads_secret || other.reads_secret,
            frozen: if rank(other.frozen) > rank(self.frozen) { other.frozen } else { self.frozen },
        }
    }

    /// Whether a grant may widen this role's WRITE face.
    pub(super) fn write_grantable(&self) -> bool {
        self.frozen != Frozen::Write && self.write_locus < LocalLocus::SystemIntegrity
    }

    /// Whether a grant may widen this role's REBIND face. Strictly stronger: freezing the write
    /// necessarily freezes the rebind, since a rebind is the more destructive of the two.
    pub(super) fn rebind_grantable(&self) -> bool {
        self.frozen == Frozen::Nothing && self.write_locus < LocalLocus::SystemIntegrity
    }
}
