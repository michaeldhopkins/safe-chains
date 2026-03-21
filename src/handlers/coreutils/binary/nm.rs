use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static NM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--debug-syms", "--defined-only", "--demangle",
        "--dynamic", "--extern-only", "--help", "--line-numbers",
        "--no-demangle", "--no-llvm-bc", "--no-sort",
        "--numeric-sort", "--portability", "--print-armap",
        "--print-file-name", "--print-size", "--reverse-sort",
        "--special-syms", "--undefined-only", "--version",
        "-A", "-B", "-C", "-D", "-P", "-S", "-V",
        "-a", "-g", "-h", "-j", "-l", "-m", "-n", "-o",
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
    FlatDef { name: "nm", policy: &NM_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/nm.1.html", aliases: &[] },
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
