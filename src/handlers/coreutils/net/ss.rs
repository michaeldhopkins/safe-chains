use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--dccp", "--extended", "--family",
        "--help", "--info", "--ipv4", "--ipv6", "--listening", "--memory",
        "--no-header", "--numeric", "--oneline", "--options",
        "--packet", "--processes", "--raw", "--resolve",
        "--sctp", "--summary", "--tcp", "--tipc", "--udp",
        "--unix", "--version", "--vsock",
        "-0", "-4", "-6", "-E", "-H", "-O", "-V",
        "-a", "-e", "-h", "-i", "-l", "-m", "-n", "-o",
        "-p", "-r", "-s", "-t", "-u", "-w", "-x",
    ]),
    valued: WordSet::flags(&[
        "--filter", "--query",
        "-A", "-F", "-f",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "ss", policy: &SS_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man8/ss.8.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        ss_bare: "ss",
        ss_listen: "ss -tlnp",
    }

    denied! {
        ss_kill_denied: "ss --kill",
        ss_kill_short_denied: "ss -K",
        ss_diag_denied: "ss -D /tmp/dump",
        ss_diag_long_denied: "ss --diag=/tmp/dump",
    }
}
