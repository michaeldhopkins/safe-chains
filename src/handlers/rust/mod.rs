mod cargo;
mod rustup;

use crate::parse::{Segment, Token};

#[cfg(test)]
pub(crate) use cargo::CARGO;
pub(crate) use rustup::RUSTUP;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    cargo::dispatch(cmd, tokens, is_safe)
        .or_else(|| RUSTUP.dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = cargo::command_docs();
    docs.extend(vec![RUSTUP.to_doc()]);
    docs
}
