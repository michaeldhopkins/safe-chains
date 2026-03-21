use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static UNIQ_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--count", "--help", "--ignore-case", "--repeated", "--unique",
        "--version", "--zero-terminated",
        "-D", "-V", "-c", "-d", "-h", "-i", "-u", "-z",
    ]),
    valued: WordSet::flags(&[
        "--all-repeated", "--check-chars", "--group", "--skip-chars",
        "--skip-fields",
        "-f", "-s", "-w",
    ]),
    bare: true,
    max_positional: Some(1),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "uniq", policy: &UNIQ_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#uniq-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        uniq_bare: "uniq",
        uniq_input: "uniq sorted.txt",
        uniq_count: "uniq -c sorted.txt",
        uniq_skip: "uniq -f 1 sorted.txt",
        uniq_ignore_case: "uniq -i sorted.txt",
        uniq_combined: "uniq -cu sorted.txt",
    }

    denied! {
        uniq_output_file_denied: "uniq input.txt output.txt",
    }
}
