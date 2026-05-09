pub(crate) mod git;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "git" => Some(git::is_safe_git(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    Vec::new()
}

