use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static WG_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static WG_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Positional,
};

static WG_SHOWCONF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: Some(1),
    flag_style: FlagStyle::Strict,
};

pub(crate) static WG: CommandDef = CommandDef {
    name: "wg",
    subs: &[
        SubDef::Policy { name: "genkey", policy: &WG_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "genpsk", policy: &WG_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "help", policy: &WG_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "pubkey", policy: &WG_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "show", policy: &WG_SHOW_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "showconf", policy: &WG_SHOWCONF_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-h"],
    url: "https://www.wireguard.com/quickstart/",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        wg_help: "wg --help",
        wg_version: "wg --version",
        wg_show: "wg show",
        wg_show_all: "wg show all",
        wg_show_iface: "wg show wg0",
        wg_show_iface_field: "wg show wg0 peers",
        wg_showconf: "wg showconf wg0",
        wg_genkey: "wg genkey",
        wg_genpsk: "wg genpsk",
        wg_pubkey: "wg pubkey",
        wg_sub_help: "wg help",
    }

    denied! {
        wg_bare_denied: "wg",
        wg_set_denied: "wg set wg0 peer abc",
        wg_setconf_denied: "wg setconf wg0 /etc/wireguard/wg0.conf",
        wg_addconf_denied: "wg addconf wg0 /etc/wireguard/wg0.conf",
        wg_syncconf_denied: "wg syncconf wg0 /etc/wireguard/wg0.conf",
    }
}
