mod fd;
mod find;

use crate::command::FlatDef;
use crate::verdict::Verdict;
use crate::parse::Token;

pub(super) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    None
        .or_else(|| find::dispatch(cmd, tokens))
        .or_else(|| fd::dispatch(cmd, tokens))
}

pub(super) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(find::command_docs());
    docs.extend(fd::command_docs());
    docs
}

pub(super) fn all_flat_defs() -> Vec<&'static FlatDef> {
    Vec::new()
}

#[cfg(test)]
pub(super) fn registry() -> Vec<&'static crate::handlers::CommandEntry> {
    let mut v = Vec::new();
    v.extend(find::REGISTRY);
    v.extend(fd::REGISTRY);
    v
}
