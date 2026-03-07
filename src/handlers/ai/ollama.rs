use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static OLLAMA_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static OLLAMA_PS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static OLLAMA_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--json", "--license", "--modelfile", "--parameters",
        "--system", "--template", "--verbose",
    ]),
    valued: WordSet::flags(&[]),
    bare: false,
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
    url: "https://github.com/ollama/ollama/blob/main/docs/api.md",
};

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
    }

    denied! {
        ollama_show_bare_denied: "ollama show",
    }
}
