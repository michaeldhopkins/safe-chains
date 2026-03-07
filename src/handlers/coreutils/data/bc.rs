use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static BC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--digit-clamp", "--global-stacks", "--interactive", "--mathlib",
        "--no-digit-clamp", "--no-line-length", "--no-prompt",
        "--no-read-prompt", "--quiet", "--standard", "--warn",
        "-C", "-P", "-R",
        "-c", "-g", "-i", "-l", "-q", "-s", "-w",
    ]),
    valued: WordSet::flags(&[
        "--expression", "--file", "--ibase", "--obase", "--redefine",
        "--scale", "--seed",
        "-E", "-I", "-O", "-S",
        "-e", "-f", "-r",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "bc", policy: &BC_POLICY, help_eligible: false, url: "https://www.gnu.org/software/bc/manual/html_mono/bc.html" },
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
