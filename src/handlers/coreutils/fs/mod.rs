mod basename;
mod bat;
mod cd;
mod colordiff;
mod date;
mod delta;
mod df;
mod diff;
mod dirname;
mod du;
mod eza;
mod file;
mod ls;
mod pwd;
mod readlink;
mod realpath;
mod stat;
mod tree;

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
    v.extend(basename::FLAT_DEFS);
    v.extend(bat::FLAT_DEFS);
    v.extend(cd::FLAT_DEFS);
    v.extend(colordiff::FLAT_DEFS);
    v.extend(date::FLAT_DEFS);
    v.extend(delta::FLAT_DEFS);
    v.extend(df::FLAT_DEFS);
    v.extend(diff::FLAT_DEFS);
    v.extend(dirname::FLAT_DEFS);
    v.extend(du::FLAT_DEFS);
    v.extend(eza::FLAT_DEFS);
    v.extend(file::FLAT_DEFS);
    v.extend(ls::FLAT_DEFS);
    v.extend(pwd::FLAT_DEFS);
    v.extend(readlink::FLAT_DEFS);
    v.extend(realpath::FLAT_DEFS);
    v.extend(stat::FLAT_DEFS);
    v.extend(tree::FLAT_DEFS);
    v
}
