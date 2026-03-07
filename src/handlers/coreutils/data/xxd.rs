use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static XXD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--autoskip", "--bits", "--capitalize", "--decimal",
        "--ebcdic", "--include", "--little-endian", "--plain",
        "--postscript", "--revert", "--uppercase",
        "-C", "-E", "-a", "-b", "-d", "-e", "-i", "-p",
        "-r", "-u",
    ]),
    valued: WordSet::flags(&[
        "--color", "--cols", "--groupsize", "--len",
        "--name", "--offset", "--seek",
        "-R", "-c", "-g", "-l", "-n", "-o", "-s",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "xxd", policy: &XXD_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/xxd.1.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        xxd_file: "xxd file.bin",
        xxd_bits: "xxd -b file.bin",
        xxd_revert: "xxd -r file.hex",
    }
}
