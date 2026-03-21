use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static IOTOP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--accumulated", "--batch", "--help", "--kilobytes", "--only",
        "--processes", "--quiet", "--version",
        "-P", "-V", "-a", "-b", "-h", "-k", "-o", "-q", "-t",
    ]),
    valued: WordSet::flags(&[
        "--delay", "--iter", "--pid", "--user",
        "-d", "-n", "-p", "-u",
    ]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "iotop", policy: &IOTOP_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man8/iotop.8.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        iotop_batch: "iotop -b -n 1",
    }
}
