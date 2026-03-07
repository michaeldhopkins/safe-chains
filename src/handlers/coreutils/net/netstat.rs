use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static NETSTAT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--continuous", "--extend", "--groups",
        "--interfaces", "--listening", "--masquerade",
        "--numeric", "--numeric-hosts", "--numeric-ports",
        "--numeric-users", "--program", "--route",
        "--statistics", "--symbolic", "--tcp", "--timers",
        "--udp", "--unix", "--verbose", "--wide",
        "-A", "-C", "-L", "-M", "-N", "-R", "-S", "-W",
        "-Z",
        "-a", "-b", "-c", "-d", "-e", "-f", "-g", "-i",
        "-l", "-m", "-n", "-o", "-p", "-q", "-r",
        "-s", "-t", "-u", "-v", "-w", "-x",
    ]),
    standalone_short: b"ACLMNRSWZabcdefgilmnopqrstuvwx",
    valued: WordSet::flags(&["-I"]),
    valued_short: b"I",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "netstat", policy: &NETSTAT_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man8/netstat.8.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        netstat_bare: "netstat",
        netstat_listen: "netstat -tlnp",
        netstat_all: "netstat -an",
    }
}
