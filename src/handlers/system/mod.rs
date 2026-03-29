mod crontab;
mod mise;
mod networksetup;
mod pmset;
mod sysctl;
mod tmux;

use crate::command::FlatDef;
use crate::verdict::Verdict;
use crate::parse::Token;

pub(crate) use mise::MISE;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    MISE.dispatch(cmd, tokens)
        .or_else(|| crontab::dispatch(cmd, tokens))
        .or_else(|| pmset::dispatch(cmd, tokens))
        .or_else(|| sysctl::dispatch(cmd, tokens))
        .or_else(|| tmux::dispatch(cmd, tokens))
        .or_else(|| networksetup::dispatch(cmd, tokens))
}

pub(crate) fn system_flat_defs() -> Vec<&'static FlatDef> {
    Vec::new()
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = vec![MISE.to_doc()];
    docs.extend(crontab::command_docs());
    docs.extend(pmset::command_docs());
    docs.extend(sysctl::command_docs());
    docs.extend(networksetup::command_docs());
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
