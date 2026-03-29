mod xcrun;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    xcrun::dispatch(cmd, tokens)
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(xcrun::command_docs());
    docs
}

pub(crate) fn xcbeautify_flat_defs() -> &'static [crate::command::FlatDef] {
    &[]
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(xcrun::REGISTRY);
    v
}
