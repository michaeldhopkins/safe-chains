use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static WHOIS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-A", "-B", "-G", "-H", "-I", "-K", "-L",
        "-M", "-Q", "-R", "-S", "-a", "-b", "-c",
        "-d", "-f", "-g", "-l", "-m", "-r", "-x",
    ]),
    standalone_short: b"ABGHIKLMQRSabcdfglmrx",
    valued: WordSet::new(&[
        "-T", "-V", "-h", "-i", "-p", "-s", "-t",
    ]),
    valued_short: b"TVhipst",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "whois", policy: &WHOIS_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        whois_domain: "whois example.com",
    }
}
