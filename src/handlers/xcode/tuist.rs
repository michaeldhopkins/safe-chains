use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static TUIST_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TUIST_DUMP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--json", "--verbose", "-h"]),
    valued: WordSet::flags(&["--path", "-p"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TUIST_GRAPH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--json", "--verbose", "-h"]),
    valued: WordSet::flags(&["--format", "--path", "-f", "-p"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TUIST_INSPECT_SUB_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--json", "--verbose", "-h"]),
    valued: WordSet::flags(&["--path", "-p"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TUIST_HASH_SUB_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--json", "--verbose", "-h"]),
    valued: WordSet::flags(&["--path", "-p"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TUIST_SCAFFOLD_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--json", "-h"]),
    valued: WordSet::flags(&["--path", "-p"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TUIST_MIGRATION_SUB_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&["--path", "-p"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static TUIST: CommandDef = CommandDef {
    name: "tuist",
    subs: &[
        SubDef::Policy { name: "dump", policy: &TUIST_DUMP_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "graph", policy: &TUIST_GRAPH_POLICY, level: SafetyLevel::Inert },
        SubDef::Nested {
            name: "hash",
            subs: &[
                SubDef::Policy { name: "cache", policy: &TUIST_HASH_SUB_POLICY, level: SafetyLevel::Inert },
                SubDef::Policy { name: "selective-testing", policy: &TUIST_HASH_SUB_POLICY, level: SafetyLevel::Inert },
            ],
        },
        SubDef::Nested {
            name: "inspect",
            subs: &[
                SubDef::Policy { name: "build", policy: &TUIST_INSPECT_SUB_POLICY, level: SafetyLevel::Inert },
                SubDef::Policy { name: "bundle", policy: &TUIST_INSPECT_SUB_POLICY, level: SafetyLevel::Inert },
                SubDef::Policy { name: "dependencies", policy: &TUIST_INSPECT_SUB_POLICY, level: SafetyLevel::Inert },
                SubDef::Policy { name: "implicit-imports", policy: &TUIST_INSPECT_SUB_POLICY, level: SafetyLevel::Inert },
                SubDef::Policy { name: "redundant-imports", policy: &TUIST_INSPECT_SUB_POLICY, level: SafetyLevel::Inert },
                SubDef::Policy { name: "test", policy: &TUIST_INSPECT_SUB_POLICY, level: SafetyLevel::Inert },
            ],
        },
        SubDef::Nested {
            name: "migration",
            subs: &[
                SubDef::Policy { name: "check-empty-settings", policy: &TUIST_MIGRATION_SUB_POLICY, level: SafetyLevel::Inert },
                SubDef::Policy { name: "list-targets", policy: &TUIST_MIGRATION_SUB_POLICY, level: SafetyLevel::Inert },
            ],
        },
        SubDef::Nested {
            name: "scaffold",
            subs: &[
                SubDef::Policy { name: "list", policy: &TUIST_SCAFFOLD_LIST_POLICY, level: SafetyLevel::Inert },
            ],
        },
        SubDef::Policy { name: "version", policy: &TUIST_BARE_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://docs.tuist.dev/en/cli/",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        tuist_dump: "tuist dump",
        tuist_dump_path: "tuist dump --path /tmp/proj",
        tuist_dump_json: "tuist dump --json",
        tuist_dump_verbose: "tuist dump --verbose",
        tuist_graph: "tuist graph",
        tuist_graph_format: "tuist graph --format png",
        tuist_graph_format_path: "tuist graph --format json --path /tmp/proj",
        tuist_inspect_implicit: "tuist inspect implicit-imports",
        tuist_inspect_redundant: "tuist inspect redundant-imports",
        tuist_inspect_deps: "tuist inspect dependencies",
        tuist_inspect_build: "tuist inspect build",
        tuist_inspect_test: "tuist inspect test",
        tuist_inspect_bundle: "tuist inspect bundle",
        tuist_inspect_path: "tuist inspect implicit-imports --path /tmp/proj",
        tuist_hash_cache: "tuist hash cache",
        tuist_hash_selective: "tuist hash selective-testing",
        tuist_hash_cache_json: "tuist hash cache --json",
        tuist_scaffold_list: "tuist scaffold list",
        tuist_scaffold_list_json: "tuist scaffold list --json",
        tuist_migration_list_targets: "tuist migration list-targets",
        tuist_migration_check_empty: "tuist migration check-empty-settings",
        tuist_migration_path: "tuist migration list-targets --path /tmp/proj",
        tuist_version: "tuist version",
    }

    denied! {
        tuist_bare_denied: "tuist",
        tuist_generate_denied: "tuist generate",
        tuist_build_denied: "tuist build",
        tuist_scaffold_bare_denied: "tuist scaffold",
        tuist_inspect_bare_denied: "tuist inspect",
    }
}
