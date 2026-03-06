mod branchdiff;
mod cloc;
mod cucumber;
mod identify;
mod man;
mod safe_chains;
mod shellcheck;
mod tokei;

use crate::command::FlatDef;
use crate::parse::{Segment, Token};

pub(super) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    for flat in all_flat_defs() {
        if let r @ Some(_) = flat.dispatch(cmd, tokens) {
            return r;
        }
    }
    safe_chains::dispatch(cmd, tokens, is_safe)
}

pub(super) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs: Vec<_> = all_flat_defs().iter().map(|d| d.to_doc()).collect();
    docs.extend(safe_chains::command_docs());
    docs
}

pub(super) fn all_flat_defs() -> Vec<&'static FlatDef> {
    let mut v = Vec::new();
    v.extend(branchdiff::FLAT_DEFS);
    v.extend(cloc::FLAT_DEFS);
    v.extend(cucumber::FLAT_DEFS);
    v.extend(identify::FLAT_DEFS);
    v.extend(man::FLAT_DEFS);
    v.extend(shellcheck::FLAT_DEFS);
    v.extend(tokei::FLAT_DEFS);
    v
}

#[cfg(test)]
pub(super) fn registry() -> Vec<&'static crate::handlers::CommandEntry> {
    let mut v = Vec::new();
    v.extend(safe_chains::REGISTRY);
    v
}
