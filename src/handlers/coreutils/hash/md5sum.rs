use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static GNU_HASH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--binary", "--check", "--help", "--ignore-missing", "--quiet",
        "--status", "--strict", "--tag", "--text", "--version", "--warn",
        "--zero",
        "-V", "-b", "-c", "-h", "-t", "-w", "-z",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "md5sum", policy: &GNU_HASH_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#md5sum-invocation", aliases: &[] },
    FlatDef { name: "sha1sum", policy: &GNU_HASH_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#sha1sum-invocation", aliases: &[] },
    FlatDef { name: "sha256sum", policy: &GNU_HASH_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#sha2-utilities", aliases: &[] },
    FlatDef { name: "sha512sum", policy: &GNU_HASH_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#sha2-utilities", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        md5sum_file: "md5sum file.txt",
        md5sum_check: "md5sum -c checksums.md5",
        sha256sum_file: "sha256sum file.txt",
        sha1sum_file: "sha1sum file.txt",
        sha512sum_file: "sha512sum file.txt",
    }
}
