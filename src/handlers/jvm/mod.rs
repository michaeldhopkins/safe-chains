mod gradle;
mod jar;
mod jarsigner;
mod keytool;
mod mvn;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) use gradle::GRADLE;
pub(crate) use keytool::KEYTOOL;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    GRADLE.dispatch(cmd, tokens)
        .or_else(|| KEYTOOL.dispatch(cmd, tokens))
        .or_else(|| mvn::dispatch(cmd, tokens))
        .or_else(|| jar::dispatch(cmd, tokens))
        .or_else(|| jarsigner::dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = vec![GRADLE.to_doc()];
    docs.extend(mvn::command_docs());
    docs.push(KEYTOOL.to_doc());
    docs.extend(jar::command_docs());
    docs.extend(jarsigner::command_docs());
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
    v.extend(jarsigner::REGISTRY);
    v
}
