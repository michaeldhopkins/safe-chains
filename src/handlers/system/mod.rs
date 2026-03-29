mod asdf;
mod brew;
mod cmake;
mod crontab;
mod csrutil;
mod dcli;
mod ddev;
mod defaults;
mod diskutil;
mod fastlane;
mod firebase;
mod flyctl;
mod heroku;
mod launchctl;
mod log_cmd;
mod mise;
mod networksetup;
mod overmind;
mod pmset;
mod security;
mod sysctl;
mod tailscale;
mod terraform;
mod tmux;
mod vercel;
mod wg;

use crate::command::FlatDef;
use crate::verdict::Verdict;
use crate::parse::Token;

pub(crate) use asdf::ASDF;
pub(crate) use brew::BREW;
pub(crate) use cmake::CMAKE;
pub(crate) use csrutil::CSRUTIL;
pub(crate) use dcli::DCLI;
pub(crate) use ddev::DDEV;
pub(crate) use defaults::DEFAULTS;
pub(crate) use diskutil::DISKUTIL;
pub(crate) use flyctl::FLYCTL;
pub(crate) use heroku::HEROKU;
pub(crate) use launchctl::LAUNCHCTL;
pub(crate) use log_cmd::LOG;
pub(crate) use mise::MISE;
pub(crate) use security::SECURITY;
pub(crate) use terraform::TERRAFORM;
pub(crate) use fastlane::FASTLANE;
pub(crate) use firebase::FIREBASE;
pub(crate) use overmind::OVERMIND;
pub(crate) use tailscale::TAILSCALE;
pub(crate) use vercel::VERCEL;
pub(crate) use wg::WG;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    BREW.dispatch(cmd, tokens)
        .or_else(|| MISE.dispatch(cmd, tokens))
        .or_else(|| ASDF.dispatch(cmd, tokens))
        .or_else(|| DDEV.dispatch(cmd, tokens))
        .or_else(|| DEFAULTS.dispatch(cmd, tokens))
        .or_else(|| SECURITY.dispatch(cmd, tokens))
        .or_else(|| CSRUTIL.dispatch(cmd, tokens))
        .or_else(|| DISKUTIL.dispatch(cmd, tokens))
        .or_else(|| LAUNCHCTL.dispatch(cmd, tokens))
        .or_else(|| LOG.dispatch(cmd, tokens))
        .or_else(|| CMAKE.dispatch(cmd, tokens))
        .or_else(|| DCLI.dispatch(cmd, tokens))
        .or_else(|| TERRAFORM.dispatch(cmd, tokens))
        .or_else(|| HEROKU.dispatch(cmd, tokens))
        .or_else(|| VERCEL.dispatch(cmd, tokens))
        .or_else(|| FLYCTL.dispatch(cmd, tokens))
        .or_else(|| FASTLANE.dispatch(cmd, tokens))
        .or_else(|| FIREBASE.dispatch(cmd, tokens))
        .or_else(|| crontab::dispatch(cmd, tokens))
        .or_else(|| pmset::dispatch(cmd, tokens))
        .or_else(|| sysctl::dispatch(cmd, tokens))
        .or_else(|| OVERMIND.dispatch(cmd, tokens))
        .or_else(|| TAILSCALE.dispatch(cmd, tokens))
        .or_else(|| WG.dispatch(cmd, tokens))
        .or_else(|| tmux::dispatch(cmd, tokens))
        .or_else(|| networksetup::dispatch(cmd, tokens))
}

pub(crate) fn system_flat_defs() -> Vec<&'static FlatDef> {
    Vec::new()
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = vec![
        BREW.to_doc(),
        MISE.to_doc(),
        ASDF.to_doc(),
        DEFAULTS.to_doc(),
    ];
    docs.push(DDEV.to_doc());
    docs.extend(crontab::command_docs());
    docs.extend(pmset::command_docs());
    docs.extend(sysctl::command_docs());
    docs.push(CMAKE.to_doc());
    docs.push(DCLI.to_doc());
    docs.push(SECURITY.to_doc());
    docs.push(CSRUTIL.to_doc());
    docs.push(DISKUTIL.to_doc());
    docs.push(LAUNCHCTL.to_doc());
    docs.extend(networksetup::command_docs());
    docs.push(LOG.to_doc());
    docs.push(FASTLANE.to_doc());
    docs.push(FIREBASE.to_doc());
    docs.push(TERRAFORM.to_doc());
    docs.push(HEROKU.to_doc());
    docs.push(VERCEL.to_doc());
    docs.push(FLYCTL.to_doc());
    docs.push(OVERMIND.to_doc());
    docs.push(TAILSCALE.to_doc());
    docs.push(WG.to_doc());
    docs.extend(tmux::command_docs());
    docs
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(crontab::REGISTRY);
    v.extend(pmset::REGISTRY);
    v.extend(sysctl::REGISTRY);
    v.extend(networksetup::REGISTRY);
    v.extend(tmux::REGISTRY);
    v
}
