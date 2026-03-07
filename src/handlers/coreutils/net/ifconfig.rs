use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static IFCONFIG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["-L", "-a", "-l", "-s", "-v"]),
    standalone_short: b"Lalsv",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(1),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "ifconfig", policy: &IFCONFIG_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man8/ifconfig.8.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        ifconfig_bare: "ifconfig",
        ifconfig_iface: "ifconfig eth0",
        ifconfig_lo: "ifconfig lo0",
        ifconfig_all: "ifconfig -a",
        ifconfig_short: "ifconfig -s",
        ifconfig_verbose: "ifconfig -v",
        ifconfig_list: "ifconfig -l",
    }

    denied! {
        ifconfig_up_denied: "ifconfig eth0 up",
        ifconfig_down_denied: "ifconfig eth0 down",
        ifconfig_set_ip_denied: "ifconfig eth0 192.168.1.1",
        ifconfig_netmask_denied: "ifconfig eth0 192.168.1.1 netmask 255.255.255.0",
        ifconfig_mtu_denied: "ifconfig eth0 mtu 1500",
        ifconfig_promisc_denied: "ifconfig eth0 promisc",
    }
}
