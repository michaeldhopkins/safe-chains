use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static TR_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--complement", "--delete", "--help", "--squeeze-repeats", "--truncate-set1", "--version",
        "-C", "-V", "-c", "-d", "-h", "-s",
    ]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "tr", policy: &TR_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#tr-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        tr_lower: "tr A-Z a-z",
        tr_delete: "tr -d '\\n'",
        tr_squeeze: "tr -s ' '",
    }
}
