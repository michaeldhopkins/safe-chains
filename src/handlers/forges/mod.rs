mod gh;
mod glab;
mod jjpr;
mod tea;

use crate::parse::{Segment, Token};

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    gh::dispatch(cmd, tokens, is_safe)
        .or_else(|| glab::dispatch(cmd, tokens, is_safe))
        .or_else(|| jjpr::dispatch(cmd, tokens, is_safe))
        .or_else(|| tea::dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = gh::command_docs();
    docs.extend(glab::command_docs());
    docs.extend(jjpr::command_docs());
    docs.extend(tea::command_docs());
    docs
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(gh::REGISTRY);
    v.extend(glab::REGISTRY);
    v.extend(jjpr::REGISTRY);
    v.extend(tea::REGISTRY);
    v
}
