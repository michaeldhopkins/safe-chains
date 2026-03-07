use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DIRNAME_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--zero", "-z"]),
    standalone_short: b"z",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "dirname", policy: &DIRNAME_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#dirname-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        dirname_path: "dirname /usr/bin/ls",
        dirname_zero: "dirname -z /usr/bin/ls",
    }
}
