mod awk;
mod binary;
mod builtins;
mod data;
mod fs;
mod hash;
mod net;
mod search;
mod sed;
mod sysinfo;
mod text;
mod tools;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    None
        .or_else(|| text::dispatch(cmd, tokens))
        .or_else(|| search::dispatch(cmd, tokens))
        .or_else(|| sed::dispatch(cmd, tokens))
        .or_else(|| awk::dispatch(cmd, tokens))
        .or_else(|| data::dispatch(cmd, tokens))
        .or_else(|| hash::dispatch(cmd, tokens))
        .or_else(|| fs::dispatch(cmd, tokens))
        .or_else(|| sysinfo::dispatch(cmd, tokens))
        .or_else(|| net::dispatch(cmd, tokens))
        .or_else(|| builtins::dispatch(cmd, tokens))
        .or_else(|| binary::dispatch(cmd, tokens))
        .or_else(|| tools::dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(text::command_docs());
    docs.extend(search::command_docs());
    docs.extend(sed::command_docs());
    docs.extend(awk::command_docs());
    docs.extend(data::command_docs());
    docs.extend(hash::command_docs());
    docs.extend(fs::command_docs());
    docs.extend(sysinfo::command_docs());
    docs.extend(net::command_docs());
    docs.extend(builtins::command_docs());
    docs.extend(binary::command_docs());
    docs.extend(tools::command_docs());
    docs
}

pub(crate) fn all_flat_defs() -> Vec<&'static crate::command::FlatDef> {
    let mut v = Vec::new();
    v.extend(text::all_flat_defs());
    v.extend(search::all_flat_defs());
    v.extend(data::all_flat_defs());
    v.extend(hash::all_flat_defs());
    v.extend(fs::all_flat_defs());
    v.extend(sysinfo::all_flat_defs());
    v.extend(net::all_flat_defs());
    v.extend(builtins::all_flat_defs());
    v.extend(binary::all_flat_defs());
    v.extend(tools::all_flat_defs());
    v
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(data::registry());
    v.extend(fs::registry());
    v.extend(search::registry());
    v.extend(sed::registry());
    v.extend(awk::registry());
    v.extend(sysinfo::registry());
    v.extend(net::registry());
    v.extend(builtins::registry());
    v.extend(tools::registry());
    v
}
