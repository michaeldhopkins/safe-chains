mod codesign;
mod lipo;
mod pkgutil;
mod plutil;
mod spctl;
mod xcode_select;
mod xcodebuild;
mod xcrun;

use crate::parse::{Segment, Token};

pub(crate) use plutil::PLUTIL;
pub(crate) use xcode_select::XCODE_SELECT;
pub(crate) use xcodebuild::XCODEBUILD;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    XCODEBUILD.dispatch(cmd, tokens, is_safe)
        .or_else(|| PLUTIL.dispatch(cmd, tokens, is_safe))
        .or_else(|| XCODE_SELECT.dispatch(cmd, tokens, is_safe))
        .or_else(|| xcrun::dispatch(cmd, tokens, is_safe))
        .or_else(|| pkgutil::dispatch(cmd, tokens, is_safe))
        .or_else(|| lipo::dispatch(cmd, tokens, is_safe))
        .or_else(|| codesign::dispatch(cmd, tokens, is_safe))
        .or_else(|| spctl::dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.push(XCODEBUILD.to_doc());
    docs.push(PLUTIL.to_doc());
    docs.push(XCODE_SELECT.to_doc());
    docs.extend(xcrun::command_docs());
    docs.extend(pkgutil::command_docs());
    docs.extend(lipo::command_docs());
    docs.extend(codesign::command_docs());
    docs.extend(spctl::command_docs());
    docs
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(xcrun::REGISTRY);
    v.extend(pkgutil::REGISTRY);
    v.extend(lipo::REGISTRY);
    v.extend(codesign::REGISTRY);
    v.extend(spctl::REGISTRY);
    v
}
