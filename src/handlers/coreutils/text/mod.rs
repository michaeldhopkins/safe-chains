mod cat;
mod col;
mod column;
mod comm;
mod cut;
mod expand;
mod fmt;
mod fold;
mod head;
mod iconv;
mod nl;
mod nroff;
mod paste;
mod rev;
mod tac;
mod tail;
mod tr;
mod unexpand;
mod uniq;
mod wc;

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
    v.extend(cat::FLAT_DEFS);
    v.extend(col::FLAT_DEFS);
    v.extend(column::FLAT_DEFS);
    v.extend(comm::FLAT_DEFS);
    v.extend(cut::FLAT_DEFS);
    v.extend(expand::FLAT_DEFS);
    v.extend(fmt::FLAT_DEFS);
    v.extend(fold::FLAT_DEFS);
    v.extend(head::FLAT_DEFS);
    v.extend(iconv::FLAT_DEFS);
    v.extend(nl::FLAT_DEFS);
    v.extend(nroff::FLAT_DEFS);
    v.extend(paste::FLAT_DEFS);
    v.extend(rev::FLAT_DEFS);
    v.extend(tac::FLAT_DEFS);
    v.extend(tail::FLAT_DEFS);
    v.extend(tr::FLAT_DEFS);
    v.extend(unexpand::FLAT_DEFS);
    v.extend(uniq::FLAT_DEFS);
    v.extend(wc::FLAT_DEFS);
    v
}
