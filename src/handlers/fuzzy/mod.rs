mod fzf;
mod fzy;
mod peco;
mod pick;
mod selecta;
mod sk;
mod zf;

use crate::command::FlatDef;
use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    for flat in all_flat_defs() {
        if let r @ Some(_) = flat.dispatch(cmd, tokens) {
            return r;
        }
    }
    fzf::dispatch(cmd, tokens)
        .or_else(|| sk::dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs: Vec<_> = all_flat_defs().iter().map(|d| d.to_doc()).collect();
    docs.extend(fzf::command_docs());
    docs.extend(sk::command_docs());
    docs
}

pub(crate) fn fuzzy_flat_defs() -> Vec<&'static FlatDef> {
    let mut v = Vec::new();
    v.extend(fzy::FLAT_DEFS);
    v.extend(peco::FLAT_DEFS);
    v.extend(pick::FLAT_DEFS);
    v.extend(selecta::FLAT_DEFS);
    v.extend(zf::FLAT_DEFS);
    v
}

fn all_flat_defs() -> Vec<&'static FlatDef> {
    fuzzy_flat_defs()
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(fzf::REGISTRY);
    v.extend(sk::REGISTRY);
    v
}
