use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static LLM_MODELS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json", "--options"]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static LLM_PLUGINS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--all", "--json"]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static LLM_LOGS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--conversation", "--json", "--no-truncate", "--response", "--truncate",
    ]),
    standalone_short: b"",
    valued: WordSet::flags(&[
        "--cid", "--count", "--id", "--model", "--search",
    ]),
    valued_short: b"cnm",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static LLM_SIMPLE_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json"]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
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
    url: "https://llm.datasette.io/en/stable/",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
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
}
