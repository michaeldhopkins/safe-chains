mod gradle;
mod mvn;

use crate::parse::{Segment, Token};

pub(crate) use gradle::GRADLE;
pub(crate) use gradle::GRADLEW;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    GRADLE.dispatch(cmd, tokens, is_safe)
        .or_else(|| GRADLEW.dispatch(cmd, tokens, is_safe))
        .or_else(|| mvn::dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut doc = GRADLE.to_doc();
    doc.name = "gradle / gradlew";
    let mut docs = vec![doc];
    docs.extend(mvn::command_docs());
    docs
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(mvn::REGISTRY);
    v
}
