pub(crate) mod plutil;
pub(crate) mod sysctl;
mod tmux;

use crate::verdict::Verdict;
use crate::parse::Token;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    tmux::dispatch(cmd, tokens)
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(tmux::command_docs());
    docs
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(tmux::REGISTRY);
    v
}
