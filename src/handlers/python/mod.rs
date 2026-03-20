mod conda;
mod pip;
mod poetry;
mod pyenv;
mod uv;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) use conda::CONDA;
pub(crate) use pip::PIP;
pub(crate) use poetry::POETRY;
pub(crate) use pyenv::PYENV;
pub(crate) use uv::UV;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    PIP.dispatch(cmd, tokens)
        .or_else(|| UV.dispatch(cmd, tokens))
        .or_else(|| POETRY.dispatch(cmd, tokens))
        .or_else(|| PYENV.dispatch(cmd, tokens))
        .or_else(|| CONDA.dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = pip::command_docs();
    docs.push(UV.to_doc());
    docs.push(POETRY.to_doc());
    docs.push(PYENV.to_doc());
    docs.push(CONDA.to_doc());
    docs
}
