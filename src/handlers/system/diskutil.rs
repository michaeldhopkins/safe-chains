use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DISKUTIL_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DISKUTIL_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["-plist"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DISKUTIL_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["-all", "-plist"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static DISKUTIL: CommandDef = CommandDef {
    name: "diskutil",
    subs: &[
        SubDef::Policy { name: "list", policy: &DISKUTIL_LIST_POLICY },
        SubDef::Policy { name: "listFilesystems", policy: &DISKUTIL_LIST_POLICY },
        SubDef::Policy { name: "info", policy: &DISKUTIL_INFO_POLICY },
        SubDef::Policy { name: "activity", policy: &DISKUTIL_SIMPLE_POLICY },
        SubDef::Nested { name: "apfs", subs: &[
            SubDef::Policy { name: "list", policy: &DISKUTIL_SIMPLE_POLICY },
            SubDef::Policy { name: "listCryptoUsers", policy: &DISKUTIL_SIMPLE_POLICY },
            SubDef::Policy { name: "listSnapshots", policy: &DISKUTIL_SIMPLE_POLICY },
            SubDef::Policy { name: "listVolumeGroups", policy: &DISKUTIL_SIMPLE_POLICY },
        ]},
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://ss64.com/mac/diskutil.html",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        diskutil_list: "diskutil list",
        diskutil_list_plist: "diskutil list -plist",
        diskutil_info: "diskutil info disk0",
        diskutil_info_plist: "diskutil info -plist disk0",
        diskutil_info_all: "diskutil info -all",
        diskutil_activity: "diskutil activity",
        diskutil_list_filesystems: "diskutil listFilesystems",
        diskutil_apfs_list: "diskutil apfs list",
        diskutil_apfs_list_snapshots: "diskutil apfs listSnapshots disk1s1",
        diskutil_apfs_list_crypto: "diskutil apfs listCryptoUsers disk1s1",
        diskutil_apfs_list_volume_groups: "diskutil apfs listVolumeGroups",
    }

    denied! {
        diskutil_apfs_bare_denied: "diskutil apfs",
    }
}
