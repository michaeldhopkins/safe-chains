use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static UNAME_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--help", "--kernel-name", "--kernel-release",
        "--kernel-version", "--machine", "--nodename",
        "--operating-system", "--processor", "--version",
        "-V", "-a", "-h", "-m", "-n", "-o", "-p", "-r", "-s", "-v",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "uname", policy: &UNAME_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#uname-invocation", aliases: &[] },
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
