use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static MDFIND_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--version",
        "-0", "-V", "-count", "-h", "-interpret", "-literal", "-live",
    ]),
    valued: WordSet::flags(&["-attr", "-name", "-onlyin", "-s"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "mdfind", policy: &MDFIND_POLICY, level: SafetyLevel::Inert, url: "https://ss64.com/mac/mdfind.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        mdfind_query: "mdfind 'kMDItemContentType == public.image'",
        mdfind_name: "mdfind -name README",
    }
}
