mod handler;

use crate::parse::{Segment, Token};

pub(super) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    handler::dispatch(cmd, tokens, is_safe)
}

pub(super) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    handler::command_docs()
}

#[cfg(test)]
pub(super) fn registry() -> Vec<&'static crate::handlers::CommandEntry> {
    let mut v = Vec::new();
    v.extend(handler::REGISTRY);
    v
}
