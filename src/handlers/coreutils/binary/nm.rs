use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static NM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--debug-syms", "--defined-only", "--demangle",
        "--dynamic", "--extern-only", "--line-numbers",
        "--no-demangle", "--no-llvm-bc", "--no-sort",
        "--numeric-sort", "--portability", "--print-armap",
        "--print-file-name", "--print-size", "--reverse-sort",
        "--special-syms", "--undefined-only",
        "-A", "-B", "-C", "-D", "-P", "-S",
        "-a", "-g", "-j", "-l", "-m", "-n", "-o",
        "-p", "-r", "-s", "-u", "-v", "-x",
    ]),
    valued: WordSet::flags(&[
        "--format", "--radix", "--size-sort", "--target",
        "-f", "-t",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "nm", policy: &NM_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/nm.1.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        nm_file: "nm binary.o",
        nm_extern: "nm -g binary.o",
    }
}
