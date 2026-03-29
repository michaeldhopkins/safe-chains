mod jar;
mod mvn;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    mvn::dispatch(cmd, tokens)
        .or_else(|| jar::dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(mvn::command_docs());
    docs.extend(jar::command_docs());
    docs
}

pub(crate) fn jvm_flat_defs() -> Vec<&'static crate::command::FlatDef> {
    Vec::new()
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(mvn::REGISTRY);
    v.extend(jar::REGISTRY);
    v
}
