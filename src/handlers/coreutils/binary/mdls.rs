use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static MDLS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--raw", "--version", "-V", "-h", "-r"]),
    valued: WordSet::flags(&["--name", "--nullMarker", "-n"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "mdls", policy: &MDLS_POLICY, level: SafetyLevel::Inert, url: "https://ss64.com/mac/mdls.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        mdls_file: "mdls file.txt",
        mdls_name: "mdls -name kMDItemContentType file.txt",
    }

    denied! {
        mdls_plist_denied: "mdls -plist output.plist file.txt",
    }
}
