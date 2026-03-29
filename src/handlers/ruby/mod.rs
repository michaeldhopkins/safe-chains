mod bundle;
mod gem;
mod importmap;
mod rbenv;

use crate::command::FlatDef;
use crate::verdict::Verdict;
use crate::parse::Token;

pub(crate) use bundle::BUNDLE;
pub(crate) use gem::GEM;
pub(crate) use importmap::IMPORTMAP;
pub(crate) use rbenv::RBENV;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    BUNDLE.dispatch(cmd, tokens)
        .or_else(|| GEM.dispatch(cmd, tokens))
        .or_else(|| IMPORTMAP.dispatch(cmd, tokens))
        .or_else(|| RBENV.dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![BUNDLE.to_doc(), GEM.to_doc(), IMPORTMAP.to_doc(), RBENV.to_doc()]
}

pub(crate) fn ruby_flat_defs() -> Vec<&'static FlatDef> {
    Vec::new()
}
