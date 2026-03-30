mod gh;
mod glab;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    gh::dispatch(cmd, tokens)
        .or_else(|| glab::dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = gh::command_docs();
    docs.extend(glab::command_docs());
    docs
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(gh::REGISTRY);
    v.extend(glab::REGISTRY);
    v
}
