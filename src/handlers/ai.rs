use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

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

pub fn is_safe_ollama(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let policy = match tokens[1].as_str() {
        "list" => &OLLAMA_LIST_POLICY,
        "ps" => &OLLAMA_PS_POLICY,
        "show" => &OLLAMA_SHOW_POLICY,
        _ => return false,
    };
    policy::check(&tokens[1..], policy)
}

pub fn is_safe_llm(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let policy = match tokens[1].as_str() {
        "aliases" | "collections" | "templates" => &LLM_SIMPLE_LIST_POLICY,
        "logs" => &LLM_LOGS_POLICY,
        "models" => &LLM_MODELS_POLICY,
        "plugins" => &LLM_PLUGINS_POLICY,
        _ => return false,
    };
    policy::check(&tokens[1..], policy)
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "ollama" => Some(is_safe_ollama(tokens)),
        "llm" => Some(is_safe_llm(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("ollama",
            "Subcommands: list, ps, show."),
        CommandDoc::handler("llm",
            "Subcommands: aliases, collections, logs, models, plugins, templates. \
            "),
    ]
}

#[cfg(test)]
pub(super) const REGISTRY: &[super::CommandEntry] = &[
    super::CommandEntry::Subcommand { cmd: "ollama", subs: &[
        super::SubEntry::Policy { name: "list" },
        super::SubEntry::Policy { name: "ps" },
        super::SubEntry::Policy { name: "show" },
    ]},
    super::CommandEntry::Subcommand { cmd: "llm", subs: &[
        super::SubEntry::Policy { name: "aliases" },
        super::SubEntry::Policy { name: "collections" },
        super::SubEntry::Policy { name: "logs" },
        super::SubEntry::Policy { name: "models" },
        super::SubEntry::Policy { name: "plugins" },
        super::SubEntry::Policy { name: "templates" },
    ]},
];

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
        ollama_no_args_denied: "ollama",
        ollama_show_bare_denied: "ollama show",
        llm_no_args_denied: "llm",
    }
}
