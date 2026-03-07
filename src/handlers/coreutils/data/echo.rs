use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static ECHO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["-E", "-e", "-n"]),
    standalone_short: b"Een",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Positional,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "echo", policy: &ECHO_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#echo-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        echo_hello: "echo hello world",
        echo_no_newline: "echo -n hello",
        echo_escape: "echo -e 'hello\\nworld'",
        echo_bare: "echo",
        echo_dashes: "echo ---",
        echo_flag_like_arg: "echo --unknown hello",
    }
}
