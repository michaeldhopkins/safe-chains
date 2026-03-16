mod basename;
mod bat;
mod cd;
mod cmp;
mod colordiff;
mod date;
mod delta;
mod df;
mod diff;
mod dirname;
mod du;
mod eza;
mod file;
mod gzip;
mod ls;
mod pwd;
mod readlink;
mod realpath;
mod stat;
mod tar;
mod tree;
mod unzip;
mod zipinfo;

use crate::command::FlatDef;
use crate::parse::Token;

pub(super) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<bool> {
    for flat in all_flat_defs() {
        if let r @ Some(_) = flat.dispatch(cmd, tokens) {
            return r;
        }
    }
    None
        .or_else(|| gzip::dispatch(cmd, tokens))
        .or_else(|| tar::dispatch(cmd, tokens))
        .or_else(|| unzip::dispatch(cmd, tokens))
}

pub(super) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs: Vec<_> = all_flat_defs().iter().map(|d| d.to_doc()).collect();
    docs.extend(gzip::command_docs());
    docs.extend(tar::command_docs());
    docs.extend(unzip::command_docs());
    docs
}

pub(super) fn all_flat_defs() -> Vec<&'static FlatDef> {
    let mut v = Vec::new();
    v.extend(basename::FLAT_DEFS);
    v.extend(bat::FLAT_DEFS);
    v.extend(cd::FLAT_DEFS);
    v.extend(cmp::FLAT_DEFS);
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
    v.extend(zipinfo::FLAT_DEFS);
    v
}

#[cfg(test)]
pub(super) fn registry() -> Vec<&'static crate::handlers::CommandEntry> {
    let mut v = Vec::new();
    v.extend(gzip::REGISTRY);
    v.extend(tar::REGISTRY);
    v.extend(unzip::REGISTRY);
    v
}
