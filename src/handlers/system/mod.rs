pub(crate) mod plutil;
pub(crate) mod ssh;
pub(crate) mod sysctl;
mod tmux;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    tmux::dispatch(cmd, tokens)
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(tmux::command_docs());
    docs
}
