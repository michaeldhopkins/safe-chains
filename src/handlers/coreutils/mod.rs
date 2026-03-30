mod awk;
mod data;
mod fs;
mod net;
mod search;
mod sed;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    None
        .or_else(|| search::dispatch(cmd, tokens))
        .or_else(|| sed::dispatch(cmd, tokens))
        .or_else(|| awk::dispatch(cmd, tokens))
        .or_else(|| data::dispatch(cmd, tokens))
        .or_else(|| fs::dispatch(cmd, tokens))
        .or_else(|| net::dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(search::command_docs());
    docs.extend(sed::command_docs());
    docs.extend(awk::command_docs());
    docs.extend(data::command_docs());
    docs.extend(fs::command_docs());
    docs.extend(net::command_docs());
    docs
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(data::registry());
    v.extend(fs::registry());
    v.extend(search::registry());
    v.extend(sed::registry());
    v.extend(awk::registry());
    v.extend(net::registry());
    v
}
