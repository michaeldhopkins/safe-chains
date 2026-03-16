mod aapt2;
mod adb;
mod apkanalyzer;
mod apksigner;
mod avdmanager;
mod bundletool;
mod emulator;
mod lint;
mod sdkmanager;
mod zipalign;

use crate::parse::Token;

pub(crate) use aapt2::AAPT2;
pub(crate) use apkanalyzer::APKANALYZER;
pub(crate) use apksigner::APKSIGNER;
pub(crate) use avdmanager::AVDMANAGER;
pub(crate) use bundletool::BUNDLETOOL;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<bool> {
    adb::dispatch(cmd, tokens)
        .or_else(|| APKANALYZER.dispatch(cmd, tokens))
        .or_else(|| APKSIGNER.dispatch(cmd, tokens))
        .or_else(|| BUNDLETOOL.dispatch(cmd, tokens))
        .or_else(|| AAPT2.dispatch(cmd, tokens))
        .or_else(|| AVDMANAGER.dispatch(cmd, tokens))
        .or_else(|| zipalign::dispatch(cmd, tokens))
        .or_else(|| emulator::DEFS.iter().find_map(|d| d.dispatch(cmd, tokens)))
        .or_else(|| sdkmanager::DEFS.iter().find_map(|d| d.dispatch(cmd, tokens)))
        .or_else(|| lint::DEFS.iter().find_map(|d| d.dispatch(cmd, tokens)))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = adb::command_docs();
    docs.push(APKANALYZER.to_doc());
    docs.push(APKSIGNER.to_doc());
    docs.push(BUNDLETOOL.to_doc());
    docs.push(AAPT2.to_doc());
    docs.push(AVDMANAGER.to_doc());
    docs.extend(zipalign::command_docs());
    docs.extend(emulator::DEFS.iter().map(|d| d.to_doc()));
    docs.extend(sdkmanager::DEFS.iter().map(|d| d.to_doc()));
    docs.extend(lint::DEFS.iter().map(|d| d.to_doc()));
    docs
}

pub(crate) fn android_flat_defs() -> Vec<&'static crate::command::FlatDef> {
    emulator::DEFS.iter()
        .chain(sdkmanager::DEFS.iter())
        .chain(lint::DEFS.iter())
        .collect()
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(adb::REGISTRY);
    v.extend(zipalign::REGISTRY);
    v
}
