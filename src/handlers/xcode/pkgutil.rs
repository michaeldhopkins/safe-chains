use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

static PKGUTIL_SAFE: WordSet = WordSet::new(&[
    "--check-signature", "--export-plist",
    "--file-info", "--file-info-plist",
    "--files", "--group-pkgs", "--groups", "--groups-plist",
    "--packages", "--payload-files",
    "--pkg-groups", "--pkg-info", "--pkg-info-plist",
    "--pkgs", "--pkgs-plist",
]);

static PKGUTIL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--check-signature", "--export-plist",
        "--file-info", "--file-info-plist",
        "--files", "--group-pkgs", "--groups", "--groups-plist",
        "--packages", "--payload-files",
        "--pkg-groups", "--pkg-info", "--pkg-info-plist",
        "--pkgs", "--pkgs-plist",
        "--regexp",
    ]),
    standalone_short: b"",
    valued: WordSet::flags(&["--volume"]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_pkgutil(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if !tokens[1..].iter().any(|t| PKGUTIL_SAFE.contains(t)) {
        return false;
    }
    policy::check(tokens, &PKGUTIL_POLICY)
}

pub(in crate::handlers::xcode) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    if cmd == "pkgutil" {
        Some(is_safe_pkgutil(tokens))
    } else {
        None
    }
}

pub(in crate::handlers::xcode) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("pkgutil",
            "https://ss64.com/mac/pkgutil.html",
            "Requires a read-only flag (--pkgs, --files, --pkg-info, etc.)."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::xcode) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "pkgutil", valid_prefix: Some("pkgutil --pkgs") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        pkgutil_pkgs: "pkgutil --pkgs",
        pkgutil_files: "pkgutil --files com.apple.pkg.CLTools_Executables",
        pkgutil_pkg_info: "pkgutil --pkg-info com.apple.pkg.CLTools_Executables",
        pkgutil_check_signature: "pkgutil --check-signature /path/to/pkg",
        pkgutil_groups: "pkgutil --groups",
    }

    denied! {
        pkgutil_forget_denied: "pkgutil --forget com.example.pkg",
        pkgutil_expand_denied: "pkgutil --expand pkg.pkg /tmp/expanded",
        pkgutil_no_args_denied: "pkgutil",
    }
}
