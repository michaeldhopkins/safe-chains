use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static WHEREIS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--version", "-V", "-b", "-h", "-l", "-m", "-s", "-u"]),
    valued: WordSet::flags(&["-B", "-M", "-S", "-f"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "whereis", policy: &WHEREIS_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/whereis.1.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        whereis_cmd: "whereis ls",
    }
}
