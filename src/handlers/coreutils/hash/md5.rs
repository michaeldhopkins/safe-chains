use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static MD5_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--version", "-V", "-h", "-n", "-p", "-q", "-r", "-t"]),
    valued: WordSet::flags(&["-s"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "md5", policy: &MD5_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/md5sum.1.html", aliases: &[] },
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
