mod hexdump;
mod mdls;
mod nm;
mod od;
mod otool;
mod size;
mod strings;

use crate::command::FlatDef;
use crate::parse::{Segment, Token};

pub(super) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    for flat in all_flat_defs() {
        if let r @ Some(_) = flat.dispatch(cmd, tokens) {
            return r;
        }
    }
    None
}

pub(super) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    all_flat_defs().iter().map(|d| d.to_doc()).collect()
}

pub(super) fn all_flat_defs() -> Vec<&'static FlatDef> {
    let mut v = Vec::new();
    v.extend(hexdump::FLAT_DEFS);
    v.extend(mdls::FLAT_DEFS);
    v.extend(nm::FLAT_DEFS);
    v.extend(od::FLAT_DEFS);
    v.extend(otool::FLAT_DEFS);
    v.extend(size::FLAT_DEFS);
    v.extend(strings::FLAT_DEFS);
    v
}
