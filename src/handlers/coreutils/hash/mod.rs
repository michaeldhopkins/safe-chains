mod b2sum;
mod cksum;
mod md5;
mod md5sum;
mod shasum;
mod sum;

use crate::command::FlatDef;
use crate::verdict::Verdict;
use crate::parse::Token;

pub(super) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
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
    v.extend(b2sum::FLAT_DEFS);
    v.extend(cksum::FLAT_DEFS);
    v.extend(md5::FLAT_DEFS);
    v.extend(md5sum::FLAT_DEFS);
    v.extend(shasum::FLAT_DEFS);
    v.extend(sum::FLAT_DEFS);
    v
}
