use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static W_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--from", "--help", "--ip-addr", "--no-current", "--no-header",
        "--old-style", "--short", "--version",
        "-V", "-f", "-h", "-i", "-o", "-s", "-u",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(1),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "w", policy: &W_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/w.1.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        w_bare: "w",
        w_short: "w -s",
    }
}
