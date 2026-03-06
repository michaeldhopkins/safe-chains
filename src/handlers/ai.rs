use crate::command::{CommandDef, SubDef};
use crate::parse::{Segment, Token, WordSet};
use crate::policy::{FlagPolicy, FlagStyle};

static OLLAMA_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static OLLAMA_PS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static OLLAMA_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--json", "--license", "--modelfile", "--parameters",
        "--system", "--template", "--verbose",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static LLM_MODELS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json", "--options"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static LLM_PLUGINS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--all", "--json"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static LLM_LOGS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--conversation", "--json", "--no-truncate", "--response", "--truncate",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "--cid", "--count", "--id", "--model", "--search",
    ]),
    valued_short: b"cnm",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static LLM_SIMPLE_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static OLLAMA: CommandDef = CommandDef {
    name: "ollama",
    subs: &[
        SubDef::Policy { name: "list", policy: &OLLAMA_LIST_POLICY },
        SubDef::Policy { name: "ps", policy: &OLLAMA_PS_POLICY },
        SubDef::Policy { name: "show", policy: &OLLAMA_SHOW_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

pub(crate) static LLM: CommandDef = CommandDef {
    name: "llm",
    subs: &[
        SubDef::Policy { name: "aliases", policy: &LLM_SIMPLE_LIST_POLICY },
        SubDef::Policy { name: "collections", policy: &LLM_SIMPLE_LIST_POLICY },
        SubDef::Policy { name: "logs", policy: &LLM_LOGS_POLICY },
        SubDef::Policy { name: "models", policy: &LLM_MODELS_POLICY },
        SubDef::Policy { name: "plugins", policy: &LLM_PLUGINS_POLICY },
        SubDef::Policy { name: "templates", policy: &LLM_SIMPLE_LIST_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    OLLAMA.dispatch(cmd, tokens, is_safe)
        .or_else(|| LLM.dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![OLLAMA.to_doc(), LLM.to_doc()]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        ollama_list: "ollama list",
        ollama_list_json: "ollama list --json",
        ollama_show: "ollama show llama3",
        ollama_show_license: "ollama show llama3 --license",
        ollama_show_modelfile: "ollama show llama3 --modelfile",
        ollama_show_parameters: "ollama show llama3 --parameters",
        ollama_show_template: "ollama show llama3 --template",
        ollama_show_system: "ollama show llama3 --system",
        ollama_show_json: "ollama show llama3 --json",
        ollama_ps: "ollama ps",
        ollama_ps_json: "ollama ps --json",
        ollama_version: "ollama --version",
        llm_models: "llm models",
        llm_models_json: "llm models --json",
        llm_models_options: "llm models --options",
        llm_plugins: "llm plugins",
        llm_plugins_all: "llm plugins --all",
        llm_templates: "llm templates",
        llm_templates_json: "llm templates --json",
        llm_aliases: "llm aliases",
        llm_aliases_json: "llm aliases --json",
        llm_logs: "llm logs",
        llm_logs_count: "llm logs --count 10",
        llm_logs_json: "llm logs --json",
        llm_logs_model: "llm logs --model gpt-4",
        llm_logs_search: "llm logs --search hello",
        llm_logs_conversation: "llm logs --conversation",
        llm_collections: "llm collections",
        llm_collections_json: "llm collections --json",
        llm_version: "llm --version",
    }

    denied! {
        ollama_show_bare_denied: "ollama show",
    }
}
