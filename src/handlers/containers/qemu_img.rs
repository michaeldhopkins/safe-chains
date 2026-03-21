use crate::verdict::{SafetyLevel, Verdict};
use crate::command::{CommandDef, SubDef};
use crate::parse::{Token, WordSet};
use crate::policy::{FlagPolicy, FlagStyle};

static QEMU_IMG_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--backing-chain", "--force-share", "--help", "--image-opts", "--limits",
        "-U", "-h",
    ]),
    valued: WordSet::flags(&[
        "--cache", "--format", "--object", "--output",
        "-f", "-t",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static QEMU_IMG_CHECK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--force-share", "--help", "--image-opts", "--quiet",
        "-U", "-h", "-q",
    ]),
    valued: WordSet::flags(&[
        "--cache", "--format", "--object", "--output",
        "-T", "-f",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static QEMU_IMG_COMPARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--force-share", "--help", "--image-opts", "--progress", "--quiet", "--strict",
        "-U", "-h", "-p", "-q", "-s",
    ]),
    valued: WordSet::flags(&[
        "--a-format", "--b-format", "--cache", "--object",
        "-F", "-T", "-f",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static QEMU_IMG_MAP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--force-share", "--help", "--image-opts",
        "-U", "-h",
    ]),
    valued: WordSet::flags(&[
        "--format", "--max-length", "--object", "--output", "--start-offset",
        "-f", "-l", "-s",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static QEMU_IMG_MEASURE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--force-share", "--help", "--image-opts",
        "-U", "-h",
    ]),
    valued: WordSet::flags(&[
        "--format", "--object", "--output", "--size", "--snapshot", "--target-format",
        "-O", "-f", "-l", "-o", "-s",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static QEMU_IMG_SNAPSHOT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--force-share", "--help", "--image-opts", "--list", "--quiet",
        "-U", "-h", "-l", "-q",
    ]),
    valued: WordSet::flags(&[
        "--format", "--object",
        "-f",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn check_snapshot(tokens: &[Token]) -> Verdict {
    for t in tokens {
        let s = t.as_str();
        if s == "-a" || s == "--apply"
            || s == "-c" || s == "--create"
            || s == "-d" || s == "--delete"
        {
            return Verdict::Denied;
        }
    }
    if crate::policy::check(tokens, &QEMU_IMG_SNAPSHOT_POLICY) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }
}

static QEMU_IMG_SUBS: &[SubDef] = &[
    SubDef::Policy { name: "check", policy: &QEMU_IMG_CHECK_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "compare", policy: &QEMU_IMG_COMPARE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "info", policy: &QEMU_IMG_INFO_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "map", policy: &QEMU_IMG_MAP_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "measure", policy: &QEMU_IMG_MEASURE_POLICY, level: SafetyLevel::Inert },
    SubDef::Custom { name: "snapshot", check: check_snapshot, doc: "list-only (rejects -a, -c, -d)", test_suffix: Some("-l") },
];

pub(crate) static QEMU_IMG: CommandDef = CommandDef {
    name: "qemu-img",
    subs: QEMU_IMG_SUBS,
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://www.qemu.org/docs/master/tools/qemu-img.html",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        help: "qemu-img --help",
        version: "qemu-img --version",
        info: "qemu-img info disk.qcow2",
        info_json: "qemu-img info --output json disk.qcow2",
        info_backing: "qemu-img info --backing-chain disk.qcow2",
        info_format: "qemu-img info -f qcow2 disk.qcow2",
        info_share: "qemu-img info -U disk.qcow2",
        check_bare: "qemu-img check disk.qcow2",
        check_format: "qemu-img check -f qcow2 disk.qcow2",
        check_json: "qemu-img check --output json disk.qcow2",
        check_quiet: "qemu-img check -q disk.qcow2",
        compare: "qemu-img compare disk1.qcow2 disk2.qcow2",
        compare_strict: "qemu-img compare -s disk1.qcow2 disk2.qcow2",
        compare_formats: "qemu-img compare -f qcow2 -F raw disk1.qcow2 disk2.raw",
        map: "qemu-img map disk.qcow2",
        map_json: "qemu-img map --output json disk.qcow2",
        measure: "qemu-img measure --size 10G",
        measure_file: "qemu-img measure -f qcow2 disk.qcow2",
        measure_target: "qemu-img measure -O qcow2 --size 20G",
        snapshot_list: "qemu-img snapshot -l disk.qcow2",
        snapshot_list_long: "qemu-img snapshot --list disk.qcow2",
        snapshot_bare: "qemu-img snapshot disk.qcow2",
        info_help: "qemu-img info --help",
        check_help: "qemu-img check --help",
    }

    denied! {
        bare: "qemu-img",
        resize: "qemu-img resize disk.qcow2 20G",
        create: "qemu-img create -f qcow2 disk.qcow2 10G",
        convert: "qemu-img convert -O raw disk.qcow2 disk.raw",
        amend: "qemu-img amend -f qcow2 -o compat=1.1 disk.qcow2",
        rebase: "qemu-img rebase -b base.qcow2 disk.qcow2",
        commit: "qemu-img commit disk.qcow2",
        dd: "qemu-img dd if=disk.qcow2 of=disk.raw",
        check_repair: "qemu-img check -r all disk.qcow2",
        check_repair_long: "qemu-img check --repair leaks disk.qcow2",
        snapshot_create: "qemu-img snapshot -c snap1 disk.qcow2",
        snapshot_apply: "qemu-img snapshot -a snap1 disk.qcow2",
        snapshot_delete: "qemu-img snapshot -d snap1 disk.qcow2",
        unknown: "qemu-img unknown disk.qcow2",
    }
}
