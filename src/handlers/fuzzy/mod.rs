mod fzf;
mod sk;

use crate::command::FlatDef;
use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    fzf::dispatch(cmd, tokens)
        .or_else(|| sk::dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = fzf::command_docs();
    docs.extend(sk::command_docs());
    docs
}

pub(crate) fn fuzzy_flat_defs() -> Vec<&'static FlatDef> {
    Vec::new()
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(fzf::REGISTRY);
    v.extend(sk::REGISTRY);
    v
}
