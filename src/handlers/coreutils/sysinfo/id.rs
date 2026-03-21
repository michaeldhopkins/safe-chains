use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static ID_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--context", "--group", "--groups", "--help", "--name",
        "--real", "--user", "--version", "--zero",
        "-G", "-V", "-Z", "-g", "-h", "-n", "-p", "-r", "-u", "-z",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(1),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "id", policy: &ID_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#id-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        id_bare: "id",
        id_user: "id -u",
        id_name: "id -un",
    }
}
