mod dig;
mod host;
mod ifconfig;
mod mdfind;
mod netstat;
mod nslookup;
mod route;
mod ss;
mod whois;

use crate::command::FlatDef;
use crate::parse::{Segment, Token};

pub(super) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    for flat in all_flat_defs() {
        if let r @ Some(_) = flat.dispatch(cmd, tokens) {
            return r;
        }
    }
    None
        .or_else(|| route::dispatch(cmd, tokens, is_safe))
        .or_else(|| nslookup::dispatch(cmd, tokens, is_safe))
}

pub(super) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs: Vec<_> = all_flat_defs().iter().map(|d| d.to_doc()).collect();
    docs.extend(route::command_docs());
    docs.extend(nslookup::command_docs());
    docs
}

pub(super) fn all_flat_defs() -> Vec<&'static FlatDef> {
    let mut v = Vec::new();
    v.extend(dig::FLAT_DEFS);
    v.extend(host::FLAT_DEFS);
    v.extend(ifconfig::FLAT_DEFS);
    v.extend(mdfind::FLAT_DEFS);
    v.extend(netstat::FLAT_DEFS);
    v.extend(ss::FLAT_DEFS);
    v.extend(whois::FLAT_DEFS);
    v
}

#[cfg(test)]
pub(super) fn registry() -> Vec<&'static crate::handlers::CommandEntry> {
    let mut v = Vec::new();
    v.extend(route::REGISTRY);
    v.extend(nslookup::REGISTRY);
    v
}
