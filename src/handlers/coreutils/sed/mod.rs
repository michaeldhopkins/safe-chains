mod handler;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(super) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    handler::dispatch(cmd, tokens)
}

pub(super) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    handler::command_docs()
}

#[cfg(test)]
pub(super) fn registry() -> Vec<&'static crate::handlers::CommandEntry> {
    let mut v = Vec::new();
    v.extend(handler::REGISTRY);
    v
}
