mod dasel;
mod mlr;

use crate::verdict::Verdict;
use crate::parse::Token;

pub(super) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    dasel::dispatch(cmd, tokens)
        .or_else(|| mlr::dispatch(cmd, tokens))
}

pub(super) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(dasel::command_docs());
    docs.extend(mlr::command_docs());
    docs
}

#[cfg(test)]
pub(super) fn registry() -> Vec<&'static crate::handlers::CommandEntry> {
    let mut v = Vec::new();
    v.extend(dasel::REGISTRY);
    v.extend(mlr::REGISTRY);
    v
}
