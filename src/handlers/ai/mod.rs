mod claude;
mod hf;
mod llm;
mod ollama;

use crate::command::FlatDef;
use crate::parse::Token;

pub(crate) use hf::HF;
pub(crate) use llm::LLM;
pub(crate) use ollama::OLLAMA;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<bool> {
    for flat in ai_flat_defs() {
        if let r @ Some(_) = flat.dispatch(cmd, tokens) {
            return r;
        }
    }
    OLLAMA.dispatch(cmd, tokens)
        .or_else(|| LLM.dispatch(cmd, tokens))
        .or_else(|| HF.dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs: Vec<_> = ai_flat_defs().iter().map(|d| d.to_doc()).collect();
    docs.extend([OLLAMA.to_doc(), LLM.to_doc(), HF.to_doc()]);
    docs
}

pub(crate) fn ai_flat_defs() -> Vec<&'static FlatDef> {
    let mut v = Vec::new();
    v.extend(claude::FLAT_DEFS);
    v
}
