mod conda;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) use conda::CONDA;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    CONDA.dispatch(cmd, tokens)
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![CONDA.to_doc()]
}
