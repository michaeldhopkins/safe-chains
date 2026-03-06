use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static HOST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-4", "-6", "-C", "-a", "-c", "-d", "-l",
        "-r", "-s", "-v",
    ]),
    standalone_short: b"46Cacdlrsv",
    valued: WordSet::new(&[
        "-D", "-N", "-R", "-T", "-W", "-i", "-m", "-t",
    ]),
    valued_short: b"DNRTWimt",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "host", policy: &HOST_POLICY, help_eligible: false },
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
