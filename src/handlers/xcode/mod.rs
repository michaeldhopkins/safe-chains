mod agvtool;
mod codesign;
mod lipo;
mod periphery;
mod pkgutil;
mod plutil;
mod pod;
mod simctl;
mod spctl;
mod swiftformat;
mod swiftlint;
mod tuist;
mod xcode_select;
mod xcodebuild;
mod xcodegen;
mod xcrun;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) use agvtool::AGVTOOL;
pub(crate) use periphery::PERIPHERY;
pub(crate) use plutil::PLUTIL;
pub(crate) use pod::POD;
pub(crate) use simctl::SIMCTL;
pub(crate) use swiftlint::SWIFTLINT;
pub(crate) use tuist::TUIST;
pub(crate) use xcode_select::XCODE_SELECT;
pub(crate) use xcodebuild::XCODEBUILD;
pub(crate) use xcodegen::XCODEGEN;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    XCODEBUILD.dispatch(cmd, tokens)
        .or_else(|| PLUTIL.dispatch(cmd, tokens))
        .or_else(|| XCODE_SELECT.dispatch(cmd, tokens))
        .or_else(|| XCODEGEN.dispatch(cmd, tokens))
        .or_else(|| TUIST.dispatch(cmd, tokens))
        .or_else(|| POD.dispatch(cmd, tokens))
        .or_else(|| SWIFTLINT.dispatch(cmd, tokens))
        .or_else(|| PERIPHERY.dispatch(cmd, tokens))
        .or_else(|| AGVTOOL.dispatch(cmd, tokens))
        .or_else(|| SIMCTL.dispatch(cmd, tokens))
        .or_else(|| xcrun::dispatch(cmd, tokens))
        .or_else(|| pkgutil::dispatch(cmd, tokens))
        .or_else(|| lipo::dispatch(cmd, tokens))
        .or_else(|| codesign::dispatch(cmd, tokens))
        .or_else(|| spctl::dispatch(cmd, tokens))
        .or_else(|| swiftformat::dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = vec![
        XCODEBUILD.to_doc(),
        PLUTIL.to_doc(),
        XCODE_SELECT.to_doc(),
        XCODEGEN.to_doc(),
        TUIST.to_doc(),
        POD.to_doc(),
        SWIFTLINT.to_doc(),
        PERIPHERY.to_doc(),
        AGVTOOL.to_doc(),
        SIMCTL.to_doc(),
    ];
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
