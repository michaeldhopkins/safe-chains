use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static TEST_CMD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Positional,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "test", policy: &TEST_CMD_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#test-invocation", aliases: &["["] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        test_file: "test -f file.txt",
        test_dir: "test -d /tmp",
        test_eq: "test 1 -eq 1",
        test_bare: "test",
        bracket_file: "[ -f file.txt ]",
        bracket_n: "[ -n \"$out\" ]",
        bracket_gt: "[ \"$x\" -gt 0 ]",
    }
}
