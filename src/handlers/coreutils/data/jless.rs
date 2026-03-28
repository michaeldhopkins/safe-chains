use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static JLESS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--json", "--version", "--yaml",
        "-N", "-V", "-h", "-n",
    ]),
    valued: WordSet::flags(&[
        "--mode", "--scrolloff",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "jless", policy: &JLESS_POLICY, level: SafetyLevel::Inert, url: "https://github.com/PaulJuliusMartinez/jless", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        jless_file: "jless data.json",
        jless_yaml: "jless --yaml data.yaml",
        jless_mode: "jless --mode line data.json",
    }

    denied! {
        jless_bare_denied: "jless",
    }
}
