use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DIG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--version",
        "-4", "-6", "-V", "-h", "-m", "-r", "-u", "-v",
    ]),
    valued: WordSet::flags(&[
        "-b", "-c", "-f", "-k", "-p", "-q", "-t", "-x", "-y",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "dig", policy: &DIG_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/dig.1.html", aliases: &[] },
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
