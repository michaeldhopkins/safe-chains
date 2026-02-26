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

    #[test]
    fn ollama_list() {
        assert!(check("ollama list"));
    }

    #[test]
    fn ollama_show() {
        assert!(check("ollama show llama3"));
    }

    #[test]
    fn ollama_ps() {
        assert!(check("ollama ps"));
    }

    #[test]
    fn ollama_run_denied() {
        assert!(!check("ollama run llama3"));
    }

    #[test]
    fn ollama_pull_denied() {
        assert!(!check("ollama pull llama3"));
    }

    #[test]
    fn ollama_rm_denied() {
        assert!(!check("ollama rm llama3"));
    }

    #[test]
    fn ollama_create_denied() {
        assert!(!check("ollama create mymodel"));
    }

    #[test]
    fn ollama_serve_denied() {
        assert!(!check("ollama serve"));
    }

    #[test]
    fn ollama_push_denied() {
        assert!(!check("ollama push mymodel"));
    }

    #[test]
    fn ollama_version() {
        assert!(check("ollama --version"));
    }

    #[test]
    fn ollama_no_args_denied() {
        assert!(!check("ollama"));
    }

    #[test]
    fn llm_models() {
        assert!(check("llm models"));
    }

    #[test]
    fn llm_plugins() {
        assert!(check("llm plugins"));
    }

    #[test]
    fn llm_templates() {
        assert!(check("llm templates"));
    }

    #[test]
    fn llm_aliases() {
        assert!(check("llm aliases"));
    }

    #[test]
    fn llm_logs() {
        assert!(check("llm logs"));
    }

    #[test]
    fn llm_collections() {
        assert!(check("llm collections"));
    }

    #[test]
    fn llm_prompt_denied() {
        assert!(!check("llm prompt hello"));
    }

    #[test]
    fn llm_chat_denied() {
        assert!(!check("llm chat"));
    }

    #[test]
    fn llm_keys_denied() {
        assert!(!check("llm keys set openai"));
    }

    #[test]
    fn llm_install_denied() {
        assert!(!check("llm install llm-claude-3"));
    }

    #[test]
    fn llm_embed_denied() {
        assert!(!check("llm embed -m 3-small -c hello"));
    }

    #[test]
    fn llm_version() {
        assert!(check("llm --version"));
    }

    #[test]
    fn llm_no_args_denied() {
        assert!(!check("llm"));
    }
}
