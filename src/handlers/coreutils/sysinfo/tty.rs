use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static TTY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--quiet", "--silent", "-s"]),
    standalone_short: b"s",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "tty", policy: &TTY_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#tty-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        tty_bare: "tty",
        tty_silent: "tty -s",
    }
}
