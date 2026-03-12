mod bundle;
mod gem;
mod rbenv;
mod ruby_cmd;

use crate::command::FlatDef;
use crate::parse::{Segment, Token};

pub(crate) use bundle::BUNDLE;
pub(crate) use gem::GEM;
pub(crate) use rbenv::RBENV;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    for flat in ruby_flat_defs() {
        if let r @ Some(_) = flat.dispatch(cmd, tokens) {
            return r;
        }
    }
    BUNDLE.dispatch(cmd, tokens, is_safe)
        .or_else(|| GEM.dispatch(cmd, tokens, is_safe))
        .or_else(|| RBENV.dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs: Vec<_> = ruby_flat_defs().iter().map(|d| d.to_doc()).collect();
    docs.extend([BUNDLE.to_doc(), GEM.to_doc(), RBENV.to_doc()]);
    docs
}

pub(crate) fn ruby_flat_defs() -> Vec<&'static FlatDef> {
    let mut v = Vec::new();
    v.extend(ruby_cmd::FLAT_DEFS);
    v
}
