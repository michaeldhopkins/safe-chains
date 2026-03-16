mod cargo;
mod rustup;

use crate::parse::Token;

pub(crate) use cargo::CARGO;
pub(crate) use rustup::RUSTUP;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<bool> {
    cargo::dispatch(cmd, tokens)
        .or_else(|| RUSTUP.dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = cargo::command_docs();
    docs.extend(vec![RUSTUP.to_doc()]);
    docs
}
