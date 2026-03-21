use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static CUT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--complement", "--help", "--only-delimited", "--version", "--zero-terminated",
        "-V", "-h", "-n", "-s", "-w", "-z",
    ]),
    valued: WordSet::flags(&[
        "--bytes", "--characters", "--delimiter", "--fields",
        "--output-delimiter",
        "-b", "-c", "-d", "-f",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "cut", policy: &CUT_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#cut-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        cut_fields: "cut -f 1 file.txt",
        cut_delim: "cut -d: -f1 /etc/passwd",
        cut_bytes: "cut -b 1-10 file",
        cut_complement: "cut --complement -f 1 file",
    }
}
