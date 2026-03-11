mod detekt;
mod gradle;
mod jar;
mod jarsigner;
mod javap;
mod keytool;
mod ktlint;
mod mvn;

use crate::parse::{Segment, Token};

pub(crate) use gradle::GRADLE;
pub(crate) use gradle::GRADLEW;
pub(crate) use keytool::KEYTOOL;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    GRADLE.dispatch(cmd, tokens, is_safe)
        .or_else(|| GRADLEW.dispatch(cmd, tokens, is_safe))
        .or_else(|| KEYTOOL.dispatch(cmd, tokens, is_safe))
        .or_else(|| mvn::dispatch(cmd, tokens, is_safe))
        .or_else(|| jar::dispatch(cmd, tokens, is_safe))
        .or_else(|| jarsigner::dispatch(cmd, tokens, is_safe))
        .or_else(|| detekt::DEFS.iter().find_map(|d| d.dispatch(cmd, tokens)))
        .or_else(|| ktlint::DEFS.iter().find_map(|d| d.dispatch(cmd, tokens)))
        .or_else(|| javap::DEFS.iter().find_map(|d| d.dispatch(cmd, tokens)))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut doc = GRADLE.to_doc();
    doc.name = "gradle / gradlew";
    let mut docs = vec![doc];
    docs.extend(mvn::command_docs());
    docs.push(KEYTOOL.to_doc());
    docs.extend(jar::command_docs());
    docs.extend(jarsigner::command_docs());
    docs.extend(detekt::DEFS.iter().map(|d| d.to_doc()));
    docs.extend(ktlint::DEFS.iter().map(|d| d.to_doc()));
    docs.extend(javap::DEFS.iter().map(|d| d.to_doc()));
    docs
}

#[cfg(test)]
pub(crate) fn jvm_flat_defs() -> Vec<&'static crate::command::FlatDef> {
    detekt::DEFS.iter().chain(ktlint::DEFS.iter()).chain(javap::DEFS.iter()).collect()
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(mvn::REGISTRY);
    v.extend(jar::REGISTRY);
    v.extend(jarsigner::REGISTRY);
    v
}
