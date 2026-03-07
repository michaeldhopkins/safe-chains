use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static BASENAME_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--multiple", "--zero", "-a", "-z"]),
    valued: WordSet::flags(&["--suffix", "-s"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "basename", policy: &BASENAME_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#basename-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        basename_path: "basename /usr/bin/ls",
        basename_suffix: "basename -s .rs file.rs",
    }
}
