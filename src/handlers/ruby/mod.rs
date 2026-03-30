pub mod bundle;

use crate::verdict::Verdict;
use crate::parse::Token;

pub(crate) fn dispatch(_cmd: &str, _tokens: &[Token]) -> Option<Verdict> {
    None
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    Vec::new()
}
