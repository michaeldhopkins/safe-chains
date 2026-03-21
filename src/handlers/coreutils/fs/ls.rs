use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static LS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--almost-all", "--author", "--classify",
        "--context", "--dereference", "--dereference-command-line",
        "--dereference-command-line-symlink-to-dir", "--directory",
        "--escape", "--file-type", "--full-time",
        "--group-directories-first", "--help", "--hide-control-chars",
        "--human-readable", "--indicator-style",
        "--inode", "--kibibytes", "--literal", "--no-group",
        "--numeric-uid-gid", "--quote-name", "--recursive",
        "--reverse", "--show-control-chars", "--si", "--size", "--version",
        "-1", "-A", "-B", "-C", "-F", "-G", "-H", "-L",
        "-N", "-Q", "-R", "-S", "-U", "-V", "-X", "-Z",
        "-a", "-c", "-d", "-f", "-g", "-h", "-i", "-k",
        "-l", "-m", "-n", "-o", "-p", "-q", "-r", "-s",
        "-t", "-u", "-v", "-x",
    ]),
    valued: WordSet::flags(&[
        "--block-size", "--color", "--format", "--hide",
        "--hyperlink", "--ignore",
        "--quoting-style", "--sort", "--tabsize", "--time",
        "--time-style", "--width",
        "-I", "-T", "-w",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "ls", policy: &LS_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#ls-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        ls_basic: "ls",
        ls_all: "ls -la",
        ls_recursive: "ls -R /tmp",
        ls_long: "ls -la",
        ls_human: "ls -lh /tmp",
        ls_color: "ls --color=auto",
        ls_recursive_bare: "ls -R",
        ls_sort: "ls --sort=size",
        ls_bare: "ls",
    }
}
