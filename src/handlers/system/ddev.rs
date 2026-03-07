use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DDEV_DESCRIBE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json-output", "-j"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DDEV_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json-output", "-j"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DDEV_LOGS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--follow", "--time", "--timestamps", "-f"]),
    valued: WordSet::flags(&["--service", "--tail", "-s", "-t"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DDEV_SNAPSHOT_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--all", "--list"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DDEV_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static DDEV: CommandDef = CommandDef {
    name: "ddev",
    subs: &[
        SubDef::Policy { name: "aliases", policy: &DDEV_BARE_POLICY },
        SubDef::Nested {
            name: "debug",
            subs: &[
                SubDef::Policy { name: "configyaml", policy: &DDEV_BARE_POLICY },
                SubDef::Policy { name: "diagnose", policy: &DDEV_BARE_POLICY },
                SubDef::Policy { name: "mutagen", policy: &DDEV_BARE_POLICY },
                SubDef::Policy { name: "test", policy: &DDEV_BARE_POLICY },
            ],
        },
        SubDef::Policy { name: "describe", policy: &DDEV_DESCRIBE_POLICY },
        SubDef::Policy { name: "list", policy: &DDEV_LIST_POLICY },
        SubDef::Policy { name: "logs", policy: &DDEV_LOGS_POLICY },
        SubDef::Guarded {
            name: "snapshot",
            guard_short: None,
            guard_long: "--list",
            policy: &DDEV_SNAPSHOT_LIST_POLICY,
        },
        SubDef::Policy { name: "status", policy: &DDEV_DESCRIBE_POLICY },
        SubDef::Policy { name: "version", policy: &DDEV_BARE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://ddev.readthedocs.io/en/stable/users/usage/commands/",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        ddev_describe: "ddev describe",
        ddev_describe_json: "ddev describe --json-output",
        ddev_describe_short: "ddev describe -j",
        ddev_describe_project: "ddev describe myproject",
        ddev_status: "ddev status",
        ddev_list: "ddev list",
        ddev_list_json: "ddev list --json-output",
        ddev_version: "ddev version",
        ddev_aliases: "ddev aliases",
        ddev_logs: "ddev logs",
        ddev_logs_follow: "ddev logs --follow",
        ddev_logs_service: "ddev logs --service web",
        ddev_logs_tail: "ddev logs --tail 50",
        ddev_logs_time: "ddev logs --time",
        ddev_snapshot_list: "ddev snapshot --list",
        ddev_snapshot_list_all: "ddev snapshot --list --all",
        ddev_debug_diagnose: "ddev debug diagnose",
        ddev_debug_configyaml: "ddev debug configyaml",
        ddev_debug_mutagen: "ddev debug mutagen",
        ddev_debug_test: "ddev debug test",
        ddev_help: "ddev --help",
    }

    denied! {
        ddev_bare_denied: "ddev",
        ddev_start_denied: "ddev start",
        ddev_stop_denied: "ddev stop",
        ddev_restart_denied: "ddev restart",
        ddev_delete_denied: "ddev delete",
        ddev_config_denied: "ddev config",
        ddev_exec_denied: "ddev exec ls",
        ddev_ssh_denied: "ddev ssh",
        ddev_snapshot_bare_denied: "ddev snapshot",
        ddev_snapshot_restore_denied: "ddev snapshot restore",
        ddev_import_db_denied: "ddev import-db",
        ddev_unknown_denied: "ddev xyzzy",
    }
}
