mod command;
mod export;
mod false_cmd;
mod hostname;
mod printenv;
mod read;
mod true_cmd;
mod type_cmd;
mod unset;
mod wait;
mod whereis;
mod which;
mod whoami;

use crate::command::FlatDef;
use crate::parse::{Segment, Token};

pub(super) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    for flat in all_flat_defs() {
        if let r @ Some(_) = flat.dispatch(cmd, tokens) {
            return r;
        }
    }
    None
        .or_else(|| command::dispatch(cmd, tokens, is_safe))
        .or_else(|| hostname::dispatch(cmd, tokens, is_safe))
}

pub(super) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs: Vec<_> = all_flat_defs().iter().map(|d| d.to_doc()).collect();
    docs.extend(command::command_docs());
    docs.extend(hostname::command_docs());
    docs
}

pub(super) fn all_flat_defs() -> Vec<&'static FlatDef> {
    let mut v = Vec::new();
    v.extend(export::FLAT_DEFS);
    v.extend(false_cmd::FLAT_DEFS);
    v.extend(printenv::FLAT_DEFS);
    v.extend(read::FLAT_DEFS);
    v.extend(true_cmd::FLAT_DEFS);
    v.extend(type_cmd::FLAT_DEFS);
    v.extend(unset::FLAT_DEFS);
    v.extend(wait::FLAT_DEFS);
    v.extend(whereis::FLAT_DEFS);
    v.extend(which::FLAT_DEFS);
    v.extend(whoami::FLAT_DEFS);
    v
}

#[cfg(test)]
pub(super) fn registry() -> Vec<&'static crate::handlers::CommandEntry> {
    let mut v = Vec::new();
    v.extend(command::REGISTRY);
    v.extend(hostname::REGISTRY);
    v
}
