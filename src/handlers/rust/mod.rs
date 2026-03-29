use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn dispatch(_cmd: &str, _tokens: &[Token]) -> Option<Verdict> {
    None
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    Vec::new()
}
