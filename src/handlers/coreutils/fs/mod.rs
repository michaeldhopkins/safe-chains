mod gzip;
mod tar;
mod unzip;

use crate::command::FlatDef;
use crate::verdict::Verdict;
use crate::parse::Token;

pub(super) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    None
        .or_else(|| gzip::dispatch(cmd, tokens))
        .or_else(|| tar::dispatch(cmd, tokens))
        .or_else(|| unzip::dispatch(cmd, tokens))
}

pub(super) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(gzip::command_docs());
    docs.extend(tar::command_docs());
    docs.extend(unzip::command_docs());
    docs
}

pub(super) fn all_flat_defs() -> Vec<&'static FlatDef> {
    Vec::new()
}

#[cfg(test)]
pub(super) fn registry() -> Vec<&'static crate::handlers::CommandEntry> {
    let mut v = Vec::new();
    v.extend(gzip::REGISTRY);
    v.extend(tar::REGISTRY);
    v.extend(unzip::REGISTRY);
    v
}
