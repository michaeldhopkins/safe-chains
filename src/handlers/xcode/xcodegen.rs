use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static XCODEGEN_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static XCODEGEN_DUMP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--no-env", "--quiet"]),
    standalone_short: b"nq",
    valued: WordSet::flags(&["--project-root", "--spec", "--type"]),
    valued_short: b"rst",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static XCODEGEN: CommandDef = CommandDef {
    name: "xcodegen",
    subs: &[
        SubDef::Policy { name: "dump", policy: &XCODEGEN_DUMP_POLICY },
        SubDef::Policy { name: "version", policy: &XCODEGEN_BARE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://github.com/yonaskolb/XcodeGen",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        xcodegen_dump: "xcodegen dump",
        xcodegen_dump_type: "xcodegen dump --type json",
        xcodegen_dump_spec: "xcodegen dump --spec project.yml",
        xcodegen_dump_short_flags: "xcodegen dump -t json -s project.yml -r /tmp/proj",
        xcodegen_dump_quiet: "xcodegen dump --quiet",
        xcodegen_dump_no_env: "xcodegen dump --no-env",
        xcodegen_version: "xcodegen version",
    }

    denied! {
        xcodegen_bare_denied: "xcodegen",
        xcodegen_generate_denied: "xcodegen generate",
        xcodegen_dump_file_denied: "xcodegen dump --file output.yml",
        xcodegen_dump_f_denied: "xcodegen dump -f output.yml",
    }
}
