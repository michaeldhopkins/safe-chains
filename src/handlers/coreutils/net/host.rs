use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static HOST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "-4", "-6", "-C", "-a", "-c", "-d", "-l",
        "-r", "-s", "-v",
    ]),
    valued: WordSet::flags(&[
        "-D", "-N", "-R", "-T", "-W", "-i", "-m", "-t",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "host", policy: &HOST_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/host.1.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        host_domain: "host example.com",
        host_type: "host -t AAAA example.com",
    }
}
