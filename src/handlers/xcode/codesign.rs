use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

static CODESIGN_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--deep", "--display", "--verify",
        "-R", "-d", "-v",
    ]),
    standalone_short: b"Rdv",
    valued: WordSet::new(&["--verbose"]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_codesign(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    static CODESIGN_SAFE: WordSet = WordSet::new(&["--display", "--verify", "-d", "-v"]);
    if !tokens[1..].iter().any(|t| CODESIGN_SAFE.contains(t)) {
        return false;
    }
    policy::check(tokens, &CODESIGN_POLICY)
}

pub(in crate::handlers::xcode) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    if cmd == "codesign" {
        Some(is_safe_codesign(tokens))
    } else {
        None
    }
}

pub(in crate::handlers::xcode) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("codesign",
            "Requires --display/-d or --verify/-v."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::xcode) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "codesign", valid_prefix: Some("codesign -d /usr/bin/ls") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        codesign_display: "codesign -d /Applications/Safari.app",
        codesign_display_long: "codesign --display --verbose=4 /usr/bin/ls",
        codesign_verify: "codesign -v /usr/bin/ls",
        codesign_verify_long: "codesign --verify --deep /usr/bin/ls",
    }

    denied! {
        codesign_sign_denied: "codesign -s - binary",
        codesign_remove_signature_denied: "codesign --remove-signature binary",
        codesign_force_denied: "codesign -f -s - binary",
        codesign_no_args_denied: "codesign",
    }
}
