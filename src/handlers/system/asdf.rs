use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static ASDF_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static ASDF: CommandDef = CommandDef {
    name: "asdf",
    subs: &[
        SubDef::Policy { name: "current", policy: &ASDF_SIMPLE_POLICY },
        SubDef::Policy { name: "help", policy: &ASDF_SIMPLE_POLICY },
        SubDef::Policy { name: "info", policy: &ASDF_SIMPLE_POLICY },
        SubDef::Policy { name: "list", policy: &ASDF_SIMPLE_POLICY },
        SubDef::Policy { name: "version", policy: &ASDF_SIMPLE_POLICY },
        SubDef::Policy { name: "which", policy: &ASDF_SIMPLE_POLICY },
        SubDef::Nested { name: "plugin", subs: &[
            SubDef::Policy { name: "list", policy: &ASDF_SIMPLE_POLICY },
        ]},
        SubDef::Policy { name: "plugin-list", policy: &ASDF_SIMPLE_POLICY },
        SubDef::Policy { name: "plugin-list-all", policy: &ASDF_SIMPLE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://asdf-vm.com/manage/commands.html",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        asdf_current: "asdf current ruby",
        asdf_which: "asdf which ruby",
        asdf_help: "asdf help",
        asdf_list: "asdf list ruby",
        asdf_version: "asdf --version",
        asdf_version_bare: "asdf version",
        asdf_info: "asdf info",
        asdf_plugin_list: "asdf plugin list",
        asdf_plugin_list_all: "asdf plugin list all",
        asdf_plugin_list_legacy: "asdf plugin-list",
        asdf_plugin_list_all_legacy: "asdf plugin-list-all",
    }
}
