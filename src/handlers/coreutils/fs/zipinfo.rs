use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static ZIPINFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--version",
        "-1", "-2", "-C", "-M", "-T", "-V", "-Z", "-h", "-l", "-m", "-s", "-t", "-v", "-z",
    ]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "zipinfo", policy: &ZIPINFO_POLICY, level: SafetyLevel::Inert, url: "https://linux.die.net/man/1/zipinfo", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        zipinfo_file: "zipinfo archive.zip",
        zipinfo_short: "zipinfo -1 archive.zip",
        zipinfo_long: "zipinfo -l archive.zip",
        zipinfo_verbose: "zipinfo -v archive.zip",
        zipinfo_totals: "zipinfo -t archive.zip",
        zipinfo_header: "zipinfo -h archive.zip",
    }

    denied! {
        zipinfo_bare: "zipinfo",
    }
}
