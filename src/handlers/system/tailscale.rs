use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static TS_POSITIONAL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Positional,
};

static TS_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static TS_STATUS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--active", "--browser", "--help", "--json",
        "--peers", "--self", "--web",
        "-h",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static TS_PING_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--c2c", "--help", "--icmp", "--peerapi", "--tsmp", "--until-direct", "--verbose",
        "-h",
    ]),
    valued: WordSet::flags(&[
        "--size", "--timeout",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TS_IP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--1", "--4", "--6", "--help",
        "-h",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TS_WHOIS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--json", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TS_UP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--accept-dns", "--accept-routes", "--help", "--reset", "--shields-up",
        "-h",
    ]),
    valued: WordSet::flags(&[
        "--advertise-exit-node", "--advertise-routes",
        "--advertise-tags", "--authkey", "--exit-node",
        "--exit-node-allow-lan-access", "--hostname",
        "--login-server", "--operator", "--timeout",
    ]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static TS_DOWN_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static TS_SWITCH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--list", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TS_LOGIN_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--accept-dns", "--accept-routes", "--help", "--shields-up",
        "-h",
    ]),
    valued: WordSet::flags(&[
        "--advertise-exit-node", "--advertise-routes",
        "--advertise-tags", "--authkey", "--exit-node",
        "--hostname", "--login-server", "--operator", "--timeout",
    ]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static TS_LOGOUT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static TS_DNS_SUB_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Positional,
};

static TS_EXIT_NODE_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--json", "-h"]),
    valued: WordSet::flags(&["--filter"]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(crate) static TAILSCALE: CommandDef = CommandDef {
    name: "tailscale",
    subs: &[
        SubDef::Policy { name: "bugreport", policy: &TS_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Nested { name: "dns", subs: &[
            SubDef::Policy { name: "query", policy: &TS_DNS_SUB_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "status", policy: &TS_BARE_POLICY, level: SafetyLevel::Inert },
        ]},
        SubDef::Policy { name: "down", policy: &TS_DOWN_POLICY, level: SafetyLevel::SafeWrite },
        SubDef::Nested { name: "exit-node", subs: &[
            SubDef::Policy { name: "list", policy: &TS_EXIT_NODE_LIST_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "suggest", policy: &TS_BARE_POLICY, level: SafetyLevel::Inert },
        ]},
        SubDef::Policy { name: "ip", policy: &TS_IP_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "licenses", policy: &TS_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "login", policy: &TS_LOGIN_POLICY, level: SafetyLevel::SafeWrite },
        SubDef::Policy { name: "logout", policy: &TS_LOGOUT_POLICY, level: SafetyLevel::SafeWrite },
        SubDef::Policy { name: "netcheck", policy: &TS_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "ping", policy: &TS_PING_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "status", policy: &TS_STATUS_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "switch", policy: &TS_SWITCH_POLICY, level: SafetyLevel::SafeWrite },
        SubDef::Policy { name: "up", policy: &TS_UP_POLICY, level: SafetyLevel::SafeWrite },
        SubDef::Policy { name: "version", policy: &TS_POSITIONAL_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "whois", policy: &TS_WHOIS_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-h"],
    url: "https://tailscale.com/kb/1080/cli",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        ts_help: "tailscale --help",
        ts_version: "tailscale --version",
        ts_status: "tailscale status",
        ts_status_json: "tailscale status --json",
        ts_ip: "tailscale ip",
        ts_ip_4: "tailscale ip --4",
        ts_ping: "tailscale ping 100.64.0.1",
        ts_ping_verbose: "tailscale ping --verbose 100.64.0.1",
        ts_whois: "tailscale whois 100.64.0.1",
        ts_whois_json: "tailscale whois --json 100.64.0.1",
        ts_netcheck: "tailscale netcheck",
        ts_bugreport: "tailscale bugreport",
        ts_licenses: "tailscale licenses",
        ts_dns_status: "tailscale dns status",
        ts_dns_query: "tailscale dns query example.com",
        ts_exit_node_list: "tailscale exit-node list",
        ts_exit_node_suggest: "tailscale exit-node suggest",
        ts_up: "tailscale up",
        ts_up_authkey: "tailscale up --authkey tskey-abc123",
        ts_down: "tailscale down",
        ts_login: "tailscale login",
        ts_logout: "tailscale logout",
        ts_switch: "tailscale switch user@example.com",
        ts_switch_list: "tailscale switch --list",
        ts_version_sub: "tailscale version",
    }

    denied! {
        ts_bare_denied: "tailscale",
        ts_serve_denied: "tailscale serve 3000",
        ts_funnel_denied: "tailscale funnel 3000",
        ts_cert_denied: "tailscale cert example.com",
        ts_ssh_denied: "tailscale ssh user@host",
        ts_set_denied: "tailscale set --hostname=myhost",
        ts_file_denied: "tailscale file cp file.txt peer:",
        ts_update_denied: "tailscale update",
    }

    inert! {
        level_ts_status: "tailscale status",
        level_ts_ping: "tailscale ping 100.64.0.1",
        level_ts_whois: "tailscale whois 100.64.0.1",
    }

    safe_write! {
        level_ts_up: "tailscale up",
        level_ts_down: "tailscale down",
        level_ts_login: "tailscale login",
        level_ts_logout: "tailscale logout",
    }
}
