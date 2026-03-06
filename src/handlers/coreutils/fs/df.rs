use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--human-readable", "--inodes", "--local",
        "--no-sync", "--portability", "--print-type",
        "--si", "--sync", "--total",
        "-H", "-P", "-T", "-a", "-h", "-i", "-k", "-l",
    ]),
    standalone_short: b"HPTahikl",
    valued: WordSet::new(&[
        "--block-size", "--exclude-type", "--output", "--type",
        "-B", "-t", "-x",
    ]),
    valued_short: b"Btx",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "df", policy: &DF_POLICY, help_eligible: false },
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
