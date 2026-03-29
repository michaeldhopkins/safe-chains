use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};
use crate::verdict::SafetyLevel;

static ZF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--keep-order", "--plain", "--version",
        "-0", "-h", "-k", "-p", "-v",
    ]),
    valued: WordSet::flags(&[
        "--delimiter", "--filter", "--height", "--lines",
        "--preview-width",
        "-d", "-f", "-l",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(super) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef {
        name: "zf",
        policy: &ZF_POLICY,
        level: SafetyLevel::Inert,
        url: "https://github.com/natecraddock/zf",
        aliases: &[],
    },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        zf_bare: "zf",
        zf_help: "zf --help",
        zf_version: "zf --version",
        zf_lines: "zf -l 20",
        zf_filter: "zf -f pattern",
        zf_plain: "zf --plain",
        zf_keep_order: "zf --keep-order",
        zf_height: "zf --height 40%",
        zf_delimiter: "zf -d :",
    }

    denied! {
        zf_preview: "zf --preview 'cat {}'",
    }
}
