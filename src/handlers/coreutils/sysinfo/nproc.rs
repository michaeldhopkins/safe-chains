use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static NPROC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--all"]),
    valued: WordSet::flags(&["--ignore"]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "nproc", policy: &NPROC_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#nproc-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        nproc_bare: "nproc",
        nproc_all: "nproc --all",
    }
}
