use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static UNAME_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--kernel-name", "--kernel-release",
        "--kernel-version", "--machine", "--nodename",
        "--operating-system", "--processor",
        "-a", "-m", "-n", "-o", "-p", "-r", "-s", "-v",
    ]),
    standalone_short: b"amnoprsv",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "uname", policy: &UNAME_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#uname-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        uname_all: "uname -a",
        uname_machine: "uname -m",
        uname_bare: "uname",
    }
}
