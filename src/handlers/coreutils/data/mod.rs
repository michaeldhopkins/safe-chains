mod base64;
mod bc;
mod echo;
mod expr;
mod factor;
mod getconf;
mod jq;
mod printf;
mod seq;
mod sort;
mod test_cmd;
mod uuidgen;
mod xmllint;
mod xxd;
mod yq;

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
    v.extend(base64::FLAT_DEFS);
    v.extend(bc::FLAT_DEFS);
    v.extend(echo::FLAT_DEFS);
    v.extend(expr::FLAT_DEFS);
    v.extend(factor::FLAT_DEFS);
    v.extend(getconf::FLAT_DEFS);
    v.extend(jq::FLAT_DEFS);
    v.extend(printf::FLAT_DEFS);
    v.extend(seq::FLAT_DEFS);
    v.extend(sort::FLAT_DEFS);
    v.extend(test_cmd::FLAT_DEFS);
    v.extend(uuidgen::FLAT_DEFS);
    v.extend(xmllint::FLAT_DEFS);
    v.extend(xxd::FLAT_DEFS);
    v.extend(yq::FLAT_DEFS);
    v
}
