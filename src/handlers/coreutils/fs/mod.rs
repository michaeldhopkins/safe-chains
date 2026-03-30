mod tar;

use crate::verdict::Verdict;
use crate::parse::Token;

pub(super) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    tar::dispatch(cmd, tokens)
}

pub(super) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(tar::command_docs());
    docs
}

#[cfg(test)]
pub(super) fn registry() -> Vec<&'static crate::handlers::CommandEntry> {
    let mut v = Vec::new();
    v.extend(tar::REGISTRY);
    v
}
