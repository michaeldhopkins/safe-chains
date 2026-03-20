mod git;
mod jj;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) use git::GIT;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    git::dispatch(cmd, tokens)
        .or_else(|| jj::dispatch(cmd, tokens))
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
