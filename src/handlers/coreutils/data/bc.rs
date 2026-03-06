use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static BC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--digit-clamp", "--global-stacks", "--interactive", "--mathlib",
        "--no-digit-clamp", "--no-line-length", "--no-prompt",
        "--no-read-prompt", "--quiet", "--standard", "--warn",
        "-C", "-P", "-R",
        "-c", "-g", "-i", "-l", "-q", "-s", "-w",
    ]),
    standalone_short: b"CPRcgilqsw",
    valued: WordSet::new(&[
        "--expression", "--file", "--ibase", "--obase", "--redefine",
        "--scale", "--seed",
        "-E", "-I", "-O", "-S",
        "-e", "-f", "-r",
    ]),
    valued_short: b"EIOSefr",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "bc", policy: &BC_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        bc_bare: "bc",
        bc_mathlib: "bc -l",
        bc_quiet: "bc -q",
        bc_file: "bc -l calc.bc",
    }
}
