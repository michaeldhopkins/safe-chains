use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DIG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "-4", "-6", "-m", "-r", "-u", "-v",
    ]),
    standalone_short: b"46mruv",
    valued: WordSet::flags(&[
        "-b", "-c", "-f", "-k", "-p", "-q", "-t", "-x", "-y",
    ]),
    valued_short: b"bcfkpqtxy",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "dig", policy: &DIG_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/dig.1.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        dig_domain: "dig example.com",
        dig_type: "dig -t MX example.com",
        dig_at_server: "dig @8.8.8.8 example.com",
    }
}
