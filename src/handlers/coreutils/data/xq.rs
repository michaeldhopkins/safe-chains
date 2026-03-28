use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static XQ_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--compact-output", "--exit-status", "--help",
        "--null-input", "--raw-input", "--raw-output",
        "--slurp", "--sort-keys", "--tab", "--version",
        "--xml-output",
        "-C", "-M", "-R", "-S", "-V", "-c", "-e", "-h", "-n", "-r", "-s", "-x",
    ]),
    valued: WordSet::flags(&[
        "--arg", "--argjson", "--indent",
        "--xml-dtd", "--xml-item-depth", "--xml-root",
        "-f",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "xq", policy: &XQ_POLICY, level: SafetyLevel::Inert, url: "https://github.com/kislyuk/yq", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        xq_filter: "xq '.element' file.xml",
        xq_xml_output: "xq -x '.root' file.xml",
        xq_compact: "xq -c '.data' file.xml",
    }

    denied! {
        xq_bare_denied: "xq",
        xq_inplace_denied: "xq -i '.key = \"val\"' file.xml",
    }
}
