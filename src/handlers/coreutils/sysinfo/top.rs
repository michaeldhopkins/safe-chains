use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static TOP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "-1", "-B", "-E", "-H", "-S", "-b", "-c", "-e",
        "-i",
    ]),
    standalone_short: b"1BEHSbcei",
    valued: WordSet::flags(&[
        "-F", "-O", "-U", "-d", "-f",
        "-l", "-n", "-o", "-p", "-s", "-u", "-w",
    ]),
    valued_short: b"FOUdflnopsuw",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "top", policy: &TOP_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/top.1.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        top_batch: "top -bn1",
    }
}
