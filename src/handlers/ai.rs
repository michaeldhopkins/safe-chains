use std::collections::HashSet;
use std::sync::LazyLock;

use crate::parse::Token;

static OLLAMA_READ_ONLY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["list", "show", "ps"]));

pub fn is_safe_ollama(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && OLLAMA_READ_ONLY.contains(tokens[1].as_str())
}

static LLM_READ_ONLY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["models", "plugins", "templates", "aliases", "logs", "collections"]));

pub fn is_safe_llm(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && LLM_READ_ONLY.contains(tokens[1].as_str())
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocKind};
    vec![
        CommandDoc {
            name: "ollama",
            kind: DocKind::Handler,
            description: "Allowed: list, show, ps. Denied: run, pull, rm, create, serve, push, cp.",
        },
        CommandDoc {
            name: "llm",
            kind: DocKind::Handler,
            description: "Allowed: models, plugins, templates, aliases, logs, collections. Denied: prompt, chat, keys, install, embed.",
        },
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
    fn llm_no_args_denied() {
        assert!(!check("llm"));
    }
}
