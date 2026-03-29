mod bundle;

use crate::command::FlatDef;
use crate::verdict::Verdict;
use crate::parse::Token;

pub(crate) use bundle::BUNDLE;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    BUNDLE.dispatch(cmd, tokens)
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![BUNDLE.to_doc()]
}

pub(crate) fn ruby_flat_defs() -> Vec<&'static FlatDef> {
    Vec::new()
}
