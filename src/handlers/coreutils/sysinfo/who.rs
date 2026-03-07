use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static WHO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--boot", "--count", "--dead", "--heading",
        "--login", "--lookup", "--mesg", "--message", "--process",
        "--runlevel", "--short", "--time", "--users", "--writable",
        "-H", "-S", "-T", "-a", "-b", "-d",
        "-l", "-m", "-p", "-q", "-r",
        "-s", "-t", "-u", "-w",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(2),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "who", policy: &WHO_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#who-invocation" },
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
