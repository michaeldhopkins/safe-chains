mod nslookup;
mod ping;
mod route;

use crate::command::FlatDef;
use crate::verdict::Verdict;
use crate::parse::Token;

pub(super) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    None
        .or_else(|| nslookup::dispatch(cmd, tokens))
        .or_else(|| ping::dispatch(cmd, tokens))
        .or_else(|| route::dispatch(cmd, tokens))
}

pub(super) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(nslookup::command_docs());
    docs.extend(ping::command_docs());
    docs.extend(route::command_docs());
    docs
}

pub(super) fn all_flat_defs() -> Vec<&'static FlatDef> {
    Vec::new()
}

#[cfg(test)]
pub(super) fn registry() -> Vec<&'static crate::handlers::CommandEntry> {
    let mut v = Vec::new();
    v.extend(nslookup::REGISTRY);
    v.extend(ping::REGISTRY);
    v.extend(route::REGISTRY);
    v
}
