mod git;
mod jj;

use crate::parse::{Segment, Token};

#[cfg(test)]
pub(crate) use git::GIT;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    git::dispatch(cmd, tokens, is_safe)
        .or_else(|| jj::dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = git::command_docs();
    docs.extend(jj::command_docs());
    docs
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(jj::REGISTRY);
    v
}
