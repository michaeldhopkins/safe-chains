mod codesign;
mod lipo;
mod pkgutil;
mod spctl;
mod swiftformat;
mod xcrun;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    xcrun::dispatch(cmd, tokens)
        .or_else(|| pkgutil::dispatch(cmd, tokens))
        .or_else(|| lipo::dispatch(cmd, tokens))
        .or_else(|| codesign::dispatch(cmd, tokens))
        .or_else(|| spctl::dispatch(cmd, tokens))
        .or_else(|| swiftformat::dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(xcrun::command_docs());
    docs.extend(pkgutil::command_docs());
    docs.extend(lipo::command_docs());
    docs.extend(codesign::command_docs());
    docs.extend(spctl::command_docs());
    docs.extend(swiftformat::command_docs());
    docs
}

pub(crate) fn xcbeautify_flat_defs() -> &'static [crate::command::FlatDef] {
    &[]
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(xcrun::REGISTRY);
    v.extend(pkgutil::REGISTRY);
    v.extend(lipo::REGISTRY);
    v.extend(codesign::REGISTRY);
    v.extend(spctl::REGISTRY);
    v.extend(swiftformat::REGISTRY);
    v
}
