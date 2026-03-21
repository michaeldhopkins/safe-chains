use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--help", "--human-readable", "--inodes", "--local",
        "--no-sync", "--portability", "--print-type",
        "--si", "--sync", "--total", "--version",
        "-H", "-P", "-T", "-V", "-a", "-h", "-i", "-k", "-l",
    ]),
    valued: WordSet::flags(&[
        "--block-size", "--exclude-type", "--output", "--type",
        "-B", "-t", "-x",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "df", policy: &DF_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#df-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        df_all: "df -h",
        df_human: "df -h",
        df_type: "df -t ext4",
    }
}
