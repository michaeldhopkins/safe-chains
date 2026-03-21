use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static ZCAT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--force", "--help", "--quiet", "--verbose", "--version",
        "-V", "-f", "-h", "-q", "-v",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "zcat", policy: &ZCAT_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/zcat.1.html", aliases: &["gzcat"] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        zcat_file: "zcat file.gz",
        zcat_bare: "zcat",
        zcat_verbose: "zcat -v file.gz",
        zcat_force: "zcat -f file.gz",
        gzcat_file: "gzcat file.gz",
    }
}
