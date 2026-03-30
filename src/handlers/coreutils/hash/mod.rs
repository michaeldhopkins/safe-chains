use crate::verdict::Verdict;
use crate::parse::Token;

pub(super) fn dispatch(_cmd: &str, _tokens: &[Token]) -> Option<Verdict> {
    None
}

pub(super) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    Vec::new()
}
