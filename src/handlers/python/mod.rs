mod conda;
mod pip;
mod poetry;
mod pyenv;
mod uv;

use crate::parse::{Segment, Token};

pub(crate) use conda::CONDA;
pub(crate) use pip::PIP;
pub(crate) use pip::PIP3;
pub(crate) use poetry::POETRY;
pub(crate) use pyenv::PYENV;
pub(crate) use uv::UV;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    PIP.dispatch(cmd, tokens, is_safe)
        .or_else(|| PIP3.dispatch(cmd, tokens, is_safe))
        .or_else(|| UV.dispatch(cmd, tokens, is_safe))
        .or_else(|| POETRY.dispatch(cmd, tokens, is_safe))
        .or_else(|| PYENV.dispatch(cmd, tokens, is_safe))
        .or_else(|| CONDA.dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = pip::command_docs();
    docs.push(UV.to_doc());
    docs.push(POETRY.to_doc());
    docs.push(PYENV.to_doc());
    docs.push(CONDA.to_doc());
    docs
}
