use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static MD5_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["-n", "-p", "-q", "-r", "-t"]),
    standalone_short: b"npqrt",
    valued: WordSet::flags(&["-s"]),
    valued_short: b"s",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "md5", policy: &MD5_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/md5sum.1.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        md5_file: "md5 file.txt",
        md5_string: "md5 -s hello",
    }
}
