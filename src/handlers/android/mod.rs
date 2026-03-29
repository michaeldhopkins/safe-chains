mod adb;
mod zipalign;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    adb::dispatch(cmd, tokens)
        .or_else(|| zipalign::dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = adb::command_docs();
    docs.extend(zipalign::command_docs());
    docs
}

pub(crate) fn android_flat_defs() -> Vec<&'static crate::command::FlatDef> {
    Vec::new()
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(adb::REGISTRY);
    v.extend(zipalign::REGISTRY);
    v
}
