mod aider;
mod claude;
mod codex;
mod hf;
mod llm;
mod ollama;
mod opencode;

use crate::command::FlatDef;
use crate::parse::Token;

pub(crate) use codex::CODEX;
pub(crate) use hf::HF;
pub(crate) use llm::LLM;
pub(crate) use ollama::OLLAMA;
pub(crate) use opencode::OPENCODE;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<bool> {
    for flat in ai_flat_defs() {
        if let r @ Some(_) = flat.dispatch(cmd, tokens) {
            return r;
        }
    }
    CODEX.dispatch(cmd, tokens)
        .or_else(|| HF.dispatch(cmd, tokens))
        .or_else(|| LLM.dispatch(cmd, tokens))
        .or_else(|| OLLAMA.dispatch(cmd, tokens))
        .or_else(|| OPENCODE.dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs: Vec<_> = ai_flat_defs().iter().map(|d| d.to_doc()).collect();
    docs.extend([CODEX.to_doc(), HF.to_doc(), LLM.to_doc(), OLLAMA.to_doc(), OPENCODE.to_doc()]);
    docs
}

pub(crate) fn ai_flat_defs() -> Vec<&'static FlatDef> {
    let mut v = Vec::new();
    v.extend(aider::FLAT_DEFS);
    v.extend(claude::FLAT_DEFS);
    v
}
