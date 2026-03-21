use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static WHO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--boot", "--count", "--dead", "--heading",
        "--help", "--login", "--lookup", "--mesg", "--message", "--process",
        "--runlevel", "--short", "--time", "--users", "--version", "--writable",
        "-H", "-S", "-T", "-V", "-a", "-b", "-d",
        "-h", "-l", "-m", "-p", "-q", "-r",
        "-s", "-t", "-u", "-w",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(2),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "who", policy: &WHO_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#who-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        who_bare: "who",
        who_all: "who -a",
        who_am_i: "who am i",
    }
}
