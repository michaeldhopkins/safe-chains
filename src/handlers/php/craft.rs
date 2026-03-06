use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static CRAFT_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static CRAFT_POSITIONAL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static CRAFT: CommandDef = CommandDef {
    name: "craft",
    subs: &[
        SubDef::Policy { name: "env/show", policy: &CRAFT_POSITIONAL_POLICY },
        SubDef::Policy { name: "graphql/list-schemas", policy: &CRAFT_BARE_POLICY },
        SubDef::Policy { name: "graphql/print-schema", policy: &CRAFT_POSITIONAL_POLICY },
        SubDef::Policy { name: "help", policy: &CRAFT_BARE_POLICY },
        SubDef::Policy { name: "install/check", policy: &CRAFT_BARE_POLICY },
        SubDef::Policy { name: "migrate/history", policy: &CRAFT_BARE_POLICY },
        SubDef::Policy { name: "migrate/new", policy: &CRAFT_BARE_POLICY },
        SubDef::Policy { name: "pc/diff", policy: &CRAFT_BARE_POLICY },
        SubDef::Policy { name: "pc/export", policy: &CRAFT_BARE_POLICY },
        SubDef::Policy { name: "pc/get", policy: &CRAFT_POSITIONAL_POLICY },
        SubDef::Policy { name: "plugin/list", policy: &CRAFT_BARE_POLICY },
        SubDef::Policy { name: "queue/info", policy: &CRAFT_BARE_POLICY },
        SubDef::Policy { name: "update/info", policy: &CRAFT_BARE_POLICY },
        SubDef::Policy { name: "users/list-admins", policy: &CRAFT_BARE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        craft_help: "craft help",
        craft_version: "craft --version",
        craft_env_show: "craft env/show CRAFT_ENV",
        craft_install_check: "craft install/check",
        craft_migrate_history: "craft migrate/history",
        craft_migrate_new: "craft migrate/new",
        craft_update_info: "craft update/info",
        craft_pc_diff: "craft pc/diff",
        craft_pc_export: "craft pc/export",
        craft_pc_get: "craft pc/get system.name",
        craft_graphql_list_schemas: "craft graphql/list-schemas",
        craft_graphql_print_schema: "craft graphql/print-schema mySchema",
        craft_plugin_list: "craft plugin/list",
        craft_queue_info: "craft queue/info",
        craft_users_list_admins: "craft users/list-admins",
    }

    denied! {
        craft_bare_denied: "craft",
        craft_migrate_up_denied: "craft migrate/up",
        craft_cache_flush_denied: "craft cache/flush-all",
        craft_db_backup_denied: "craft db/backup",
        craft_resave_denied: "craft resave/entries",
        craft_setup_denied: "craft setup",
        craft_unknown_denied: "craft xyzzy",
    }
}
