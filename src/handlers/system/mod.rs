mod networksetup;
mod sysctl;
mod tmux;

use crate::verdict::Verdict;
use crate::parse::Token;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    sysctl::dispatch(cmd, tokens)
        .or_else(|| tmux::dispatch(cmd, tokens))
        .or_else(|| networksetup::dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(sysctl::command_docs());
    docs.extend(networksetup::command_docs());
    docs.extend(tmux::command_docs());
    docs
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(sysctl::REGISTRY);
    v.extend(networksetup::REGISTRY);
    v.extend(tmux::REGISTRY);
    v
}
