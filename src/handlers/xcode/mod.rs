use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn dispatch(_cmd: &str, _tokens: &[Token]) -> Option<Verdict> {
    None
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    Vec::new()
}

pub(crate) fn xcbeautify_flat_defs() -> &'static [crate::command::FlatDef] {
    &[]
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    Vec::new()
}
