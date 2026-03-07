use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static HF_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static HF_POSITIONAL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static HF_LS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&["--author", "--filter", "--limit", "--search", "--sort"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static HF_COLLECTIONS_LS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&["--limit", "--owner"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static HF_DISCUSSIONS_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static HF_JOBS_LOGS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&["--tail"]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static HF: CommandDef = CommandDef {
    name: "hf",
    subs: &[
        SubDef::Policy { name: "env", policy: &HF_BARE_POLICY },
        SubDef::Policy { name: "version", policy: &HF_BARE_POLICY },
        SubDef::Nested {
            name: "cache",
            subs: &[
                SubDef::Policy { name: "ls", policy: &HF_BARE_POLICY },
                SubDef::Policy { name: "verify", policy: &HF_POSITIONAL_POLICY },
            ],
        },
        SubDef::Nested {
            name: "collections",
            subs: &[
                SubDef::Policy { name: "info", policy: &HF_POSITIONAL_POLICY },
                SubDef::Policy { name: "ls", policy: &HF_COLLECTIONS_LS_POLICY },
            ],
        },
        SubDef::Nested {
            name: "datasets",
            subs: &[
                SubDef::Policy { name: "info", policy: &HF_POSITIONAL_POLICY },
                SubDef::Policy { name: "ls", policy: &HF_LS_POLICY },
                SubDef::Policy { name: "parquet", policy: &HF_POSITIONAL_POLICY },
            ],
        },
        SubDef::Nested {
            name: "discussions",
            subs: &[
                SubDef::Policy { name: "diff", policy: &HF_DISCUSSIONS_LIST_POLICY },
                SubDef::Policy { name: "info", policy: &HF_DISCUSSIONS_LIST_POLICY },
                SubDef::Policy { name: "list", policy: &HF_DISCUSSIONS_LIST_POLICY },
            ],
        },
        SubDef::Nested {
            name: "jobs",
            subs: &[
                SubDef::Policy { name: "logs", policy: &HF_JOBS_LOGS_POLICY },
                SubDef::Policy { name: "ps", policy: &HF_BARE_POLICY },
            ],
        },
        SubDef::Nested {
            name: "models",
            subs: &[
                SubDef::Policy { name: "info", policy: &HF_POSITIONAL_POLICY },
                SubDef::Policy { name: "ls", policy: &HF_LS_POLICY },
            ],
        },
        SubDef::Nested {
            name: "spaces",
            subs: &[
                SubDef::Policy { name: "info", policy: &HF_POSITIONAL_POLICY },
                SubDef::Policy { name: "ls", policy: &HF_LS_POLICY },
            ],
        },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://huggingface.co/docs/huggingface_hub/guides/cli",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        hf_version: "hf version",
        hf_env: "hf env",
        hf_help: "hf --help",
        hf_models_ls: "hf models ls",
        hf_models_ls_search: "hf models ls --search bert",
        hf_models_ls_author: "hf models ls --author google",
        hf_models_info: "hf models info bert-base-uncased",
        hf_datasets_ls: "hf datasets ls",
        hf_datasets_info: "hf datasets info squad",
        hf_datasets_parquet: "hf datasets parquet squad",
        hf_spaces_ls: "hf spaces ls",
        hf_spaces_info: "hf spaces info gradio/hello_world",
        hf_collections_ls: "hf collections ls",
        hf_collections_ls_owner: "hf collections ls --owner google",
        hf_collections_info: "hf collections info google/some-collection",
        hf_discussions_list: "hf discussions list google/bert",
        hf_discussions_info: "hf discussions info google/bert 42",
        hf_discussions_diff: "hf discussions diff google/bert 42",
        hf_cache_ls: "hf cache ls",
        hf_cache_verify: "hf cache verify bert-base-uncased",
        hf_jobs_ps: "hf jobs ps",
        hf_jobs_logs: "hf jobs logs job-123",
        hf_jobs_logs_tail: "hf jobs logs job-123 --tail 50",
    }

    denied! {
        hf_bare_denied: "hf",
        hf_upload_denied: "hf upload model-id .",
        hf_download_denied: "hf download model-id",
        hf_repo_create_denied: "hf repo create my-model",
        hf_models_unknown_denied: "hf models create",
        hf_unknown_denied: "hf xyzzy",
    }
}
