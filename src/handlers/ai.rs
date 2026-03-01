use crate::parse::{Token, WordSet};

static OLLAMA_READ_ONLY: WordSet =
    WordSet::new(&["--version", "list", "ps", "show"]);

pub fn is_safe_ollama(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && OLLAMA_READ_ONLY.contains(&tokens[1])
}

static LLM_READ_ONLY: WordSet =
    WordSet::new(&["--version", "aliases", "collections", "logs", "models", "plugins", "templates"]);

pub fn is_safe_llm(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && LLM_READ_ONLY.contains(&tokens[1])
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::wordset("ollama", &OLLAMA_READ_ONLY),
        CommandDoc::wordset("llm", &LLM_READ_ONLY),
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        ollama_list: "ollama list",
        ollama_show: "ollama show llama3",
        ollama_ps: "ollama ps",
        ollama_version: "ollama --version",
        llm_models: "llm models",
        llm_plugins: "llm plugins",
        llm_templates: "llm templates",
        llm_aliases: "llm aliases",
        llm_logs: "llm logs",
        llm_collections: "llm collections",
        llm_version: "llm --version",
    }

    denied! {
        ollama_run_denied: "ollama run llama3",
        ollama_pull_denied: "ollama pull llama3",
        ollama_rm_denied: "ollama rm llama3",
        ollama_create_denied: "ollama create mymodel",
        ollama_serve_denied: "ollama serve",
        ollama_push_denied: "ollama push mymodel",
        ollama_no_args_denied: "ollama",
        llm_prompt_denied: "llm prompt hello",
        llm_chat_denied: "llm chat",
        llm_keys_denied: "llm keys set openai",
        llm_install_denied: "llm install llm-claude-3",
        llm_embed_denied: "llm embed -m 3-small -c hello",
        llm_no_args_denied: "llm",
    }
}
