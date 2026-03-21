use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DIRNAME_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--version", "--zero", "-V", "-h", "-z"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "dirname", policy: &DIRNAME_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#dirname-invocation", aliases: &[] },
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
