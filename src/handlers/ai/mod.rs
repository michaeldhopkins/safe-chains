mod hf;
mod llm;
mod ollama;

use crate::parse::{Segment, Token};

pub(crate) use hf::HF;
pub(crate) use llm::LLM;
pub(crate) use ollama::OLLAMA;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    OLLAMA.dispatch(cmd, tokens, is_safe)
        .or_else(|| LLM.dispatch(cmd, tokens, is_safe))
        .or_else(|| HF.dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![OLLAMA.to_doc(), LLM.to_doc(), HF.to_doc()]
}
