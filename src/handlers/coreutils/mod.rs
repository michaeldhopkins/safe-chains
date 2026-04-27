mod awk;
mod data;
mod find;
mod grep;
mod net;
mod sed;
mod tar;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    None
        .or_else(|| find::dispatch(cmd, tokens))
        .or_else(|| sed::dispatch(cmd, tokens))
        .or_else(|| awk::dispatch(cmd, tokens))
        .or_else(|| data::dispatch(cmd, tokens))
        .or_else(|| tar::dispatch(cmd, tokens))
        .or_else(|| net::dispatch(cmd, tokens))
        .or_else(|| grep::dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(find::command_docs());
    docs.extend(sed::command_docs());
    docs.extend(awk::command_docs());
    docs.extend(data::command_docs());
    docs.extend(tar::command_docs());
    docs.extend(net::command_docs());
    docs.extend(grep::command_docs());
    docs
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(data::registry());
    v.extend(tar::REGISTRY);
    v.extend(find::REGISTRY);
    v.extend(sed::REGISTRY);
    v.extend(awk::REGISTRY);
    v.extend(net::registry());
    v.extend(grep::REGISTRY);
    v
}
