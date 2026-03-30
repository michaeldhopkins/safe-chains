mod git;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) use git::GIT;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    git::dispatch(cmd, tokens)
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    git::command_docs()
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    Vec::new()
}
