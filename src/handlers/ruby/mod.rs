mod bundle;
mod gem;
mod rbenv;

use crate::parse::{Segment, Token};

pub(crate) use bundle::BUNDLE;
pub(crate) use gem::GEM;
pub(crate) use rbenv::RBENV;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    BUNDLE.dispatch(cmd, tokens, is_safe)
        .or_else(|| GEM.dispatch(cmd, tokens, is_safe))
        .or_else(|| RBENV.dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![BUNDLE.to_doc(), GEM.to_doc(), RBENV.to_doc()]
}
