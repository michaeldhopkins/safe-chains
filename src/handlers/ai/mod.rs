mod codex;
mod hf;
mod llm;
mod ollama;
mod opencode;

use crate::command::FlatDef;
use crate::verdict::Verdict;
use crate::parse::Token;

pub(crate) use codex::CODEX;
pub(crate) use hf::HF;
pub(crate) use llm::LLM;
pub(crate) use ollama::OLLAMA;
pub(crate) use opencode::OPENCODE;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    CODEX.dispatch(cmd, tokens)
        .or_else(|| HF.dispatch(cmd, tokens))
        .or_else(|| LLM.dispatch(cmd, tokens))
        .or_else(|| OLLAMA.dispatch(cmd, tokens))
        .or_else(|| OPENCODE.dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![CODEX.to_doc(), HF.to_doc(), LLM.to_doc(), OLLAMA.to_doc(), OPENCODE.to_doc()]
}

pub(crate) fn ai_flat_defs() -> Vec<&'static FlatDef> {
    Vec::new()
}
