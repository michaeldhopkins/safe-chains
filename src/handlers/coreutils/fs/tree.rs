use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static TREE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--dirsfirst", "--du", "--fromfile", "--gitignore",
        "--inodes", "--matchdirs", "--noreport",
        "--prune", "--si",
        "-A", "-C", "-D", "-F", "-J", "-N", "-Q", "-S",
        "-X", "-a", "-d", "-f", "-g", "-h", "-i", "-l",
        "-n", "-p", "-q", "-r", "-s", "-t", "-u", "-v",
        "-x",
    ]),
    standalone_short: b"ACDFJNQSXadfghilnpqrstuvx",
    valued: WordSet::flags(&[
        "--charset", "--filelimit", "--filesfrom",
        "--sort", "--timefmt",
        "-H", "-I", "-L", "-P", "-T",
    ]),
    valued_short: b"HILPT",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "tree", policy: &TREE_POLICY, help_eligible: true, url: "https://man7.org/linux/man-pages/man1/tree.1.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        tree_basic: "tree",
        tree_depth: "tree -L 3",
        tree_dirs: "tree -d",
        tree_all: "tree -a",
        tree_pattern: "tree -P '*.rs'",
        tree_json: "tree -J",
    }

    denied! {
        tree_output_denied: "tree -o tree.txt",
    }
}
