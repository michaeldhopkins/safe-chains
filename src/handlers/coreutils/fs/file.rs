use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static FILE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--brief", "--debug", "--dereference", "--extension",
        "--keep-going", "--list", "--mime", "--mime-encoding",
        "--mime-type", "--no-buffer", "--no-dereference",
        "--no-pad", "--no-sandbox", "--preserve-date",
        "--print0", "--raw", "--special-files",
        "--uncompress", "--uncompress-noreport",
        "-0", "-D", "-I", "-L", "-N", "-S", "-Z",
        "-b", "-d", "-h", "-i", "-k", "-l",
        "-n", "-p", "-r", "-s", "-z",
    ]),
    standalone_short: b"0DILNSZbdhiklnprsz",
    valued: WordSet::flags(&[
        "--exclude", "--exclude-quiet", "--files-from",
        "--magic-file", "--parameter", "--separator",
        "-F", "-P", "-e", "-f", "-m",
    ]),
    valued_short: b"FPefm",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "file", policy: &FILE_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/file.1.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        file_basic: "file README.md",
        file_brief: "file -b README.md",
        file_mime: "file --mime README.md",
        file_multiple: "file *.txt",
        file_dereference: "file -L symlink",
    }

    denied! {
        file_compile_denied: "file -C",
        file_compile_long_denied: "file --compile",
    }
}
