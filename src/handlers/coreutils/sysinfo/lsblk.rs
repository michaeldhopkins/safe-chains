use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static LSBLK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--ascii", "--bytes", "--dedup", "--discard",
        "--fs", "--help", "--inverse", "--json", "--list", "--merge",
        "--nodeps", "--noheadings", "--output-all", "--pairs",
        "--paths", "--perms", "--raw", "--scsi", "--topology",
        "--tree", "--version", "--zoned",
        "-A", "-J", "-O", "-P", "-S", "-T", "-V",
        "-a", "-b", "-d", "-f", "-h", "-i", "-l", "-m", "-n", "-p", "-r", "-s", "-t", "-z",
    ]),
    valued: WordSet::flags(&[
        "--exclude", "--include", "--output", "--sort", "--width",
        "-E", "-I", "-e", "-o", "-w", "-x",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "lsblk", policy: &LSBLK_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man8/lsblk.8.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        lsblk_bare: "lsblk",
        lsblk_all: "lsblk -a",
        lsblk_json: "lsblk --json",
        lsblk_fs: "lsblk -f",
        lsblk_output: "lsblk -o NAME,SIZE,TYPE",
        lsblk_list: "lsblk -l",
        lsblk_device: "lsblk /dev/sda",
    }
}
