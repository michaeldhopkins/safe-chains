mod asdf;
mod brew;
mod cmake;
mod csrutil;
mod defaults;
mod diskutil;
mod launchctl;
mod log_cmd;
mod mise;
mod networksetup;
mod pmset;
mod security;
mod sysctl;

use crate::parse::{Segment, Token};

pub(crate) use asdf::ASDF;
pub(crate) use brew::BREW;
pub(crate) use cmake::CMAKE;
pub(crate) use csrutil::CSRUTIL;
pub(crate) use defaults::DEFAULTS;
pub(crate) use diskutil::DISKUTIL;
pub(crate) use launchctl::LAUNCHCTL;
pub(crate) use log_cmd::LOG;
pub(crate) use mise::MISE;
pub(crate) use security::SECURITY;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    BREW.dispatch(cmd, tokens, is_safe)
        .or_else(|| MISE.dispatch(cmd, tokens, is_safe))
        .or_else(|| ASDF.dispatch(cmd, tokens, is_safe))
        .or_else(|| DEFAULTS.dispatch(cmd, tokens, is_safe))
        .or_else(|| SECURITY.dispatch(cmd, tokens, is_safe))
        .or_else(|| CSRUTIL.dispatch(cmd, tokens, is_safe))
        .or_else(|| DISKUTIL.dispatch(cmd, tokens, is_safe))
        .or_else(|| LAUNCHCTL.dispatch(cmd, tokens, is_safe))
        .or_else(|| LOG.dispatch(cmd, tokens, is_safe))
        .or_else(|| CMAKE.dispatch(cmd, tokens, is_safe))
        .or_else(|| pmset::dispatch(cmd, tokens, is_safe))
        .or_else(|| sysctl::dispatch(cmd, tokens, is_safe))
        .or_else(|| networksetup::dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = vec![
        BREW.to_doc(),
        MISE.to_doc(),
        ASDF.to_doc(),
        DEFAULTS.to_doc(),
    ];
    docs.extend(pmset::command_docs());
    docs.extend(sysctl::command_docs());
    docs.push(CMAKE.to_doc());
    docs.push(SECURITY.to_doc());
    docs.push(CSRUTIL.to_doc());
    docs.push(DISKUTIL.to_doc());
    docs.push(LAUNCHCTL.to_doc());
    docs.extend(networksetup::command_docs());
    docs.push(LOG.to_doc());
    docs
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(pmset::REGISTRY);
    v.extend(sysctl::REGISTRY);
    v.extend(networksetup::REGISTRY);
    v
}
