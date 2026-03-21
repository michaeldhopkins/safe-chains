use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static WHOIS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--version",
        "-A", "-B", "-G", "-H", "-I", "-K", "-L",
        "-M", "-Q", "-R", "-S", "-a", "-b", "-c",
        "-d", "-f", "-g", "-l", "-m", "-r", "-x",
    ]),
    valued: WordSet::flags(&[
        "-T", "-V", "-h", "-i", "-p", "-s", "-t",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "whois", policy: &WHOIS_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/whois.1.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        whois_domain: "whois example.com",
    }
}
