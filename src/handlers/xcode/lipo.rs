use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

static LIPO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-archs", "-detailed_info", "-info", "-verify_arch",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_lipo(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    static LIPO_SAFE: WordSet =
        WordSet::new(&["-archs", "-detailed_info", "-info", "-verify_arch"]);
    if !tokens[1..].iter().any(|t| LIPO_SAFE.contains(t)) {
        return false;
    }
    policy::check(tokens, &LIPO_POLICY)
}

pub(in crate::handlers::xcode) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    if cmd == "lipo" {
        Some(is_safe_lipo(tokens))
    } else {
        None
    }
}

pub(in crate::handlers::xcode) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("lipo",
            "Requires a read-only flag (-info, -archs, -detailed_info, -verify_arch)."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::xcode) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "lipo", valid_prefix: Some("lipo -info /usr/bin/ls") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        lipo_info: "lipo -info /usr/bin/ls",
        lipo_detailed_info: "lipo -detailed_info binary",
        lipo_archs: "lipo -archs binary",
        lipo_verify_arch: "lipo -verify_arch x86_64 arm64 binary",
    }

    denied! {
        lipo_create_denied: "lipo -create a.o b.o -output universal.o",
        lipo_thin_denied: "lipo -thin arm64 -output thin binary",
        lipo_no_args_denied: "lipo",
    }
}
