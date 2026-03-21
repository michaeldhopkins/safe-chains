use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SHUF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--echo", "--help", "--repeat", "--version", "--zero-terminated",
        "-V", "-e", "-h", "-r", "-z",
    ]),
    valued: WordSet::flags(&[
        "--head-count", "--input-range", "--random-source",
        "-i", "-n",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "shuf", policy: &SHUF_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#shuf-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        shuf_bare: "shuf",
        shuf_file: "shuf file.txt",
        shuf_head: "shuf -n 1 file.txt",
        shuf_echo: "shuf -e a b c",
        shuf_range: "shuf -i 1-100",
        shuf_range_head: "shuf -i 1-100 -n 5",
        shuf_repeat: "shuf -r -n 10 file.txt",
    }

    denied! {
        shuf_output_denied: "shuf -o output.txt file.txt",
        shuf_output_long_denied: "shuf --output=result.txt file.txt",
        shuf_output_long_space_denied: "shuf --output result.txt file.txt",
    }
}
