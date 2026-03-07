use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static LAST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--dns", "--fullnames", "--fulltimes", "--hostlast",
        "--ip", "--nohostname", "--system", "--time-format",
        "-F", "-R", "-a", "-d", "-i", "-w", "-x",
    ]),
    standalone_short: b"0123456789FRadiwx",
    valued: WordSet::flags(&[
        "--limit", "--present", "--since", "--time-format", "--until",
        "-f", "-n", "-p", "-s", "-t",
    ]),
    valued_short: b"fnpst",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "last", policy: &LAST_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/last.1.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        last_bare: "last",
        last_n: "last -n 5",
        last_numeric: "last -5",
        last_file: "last -f /var/log/wtmp",
    }
}
