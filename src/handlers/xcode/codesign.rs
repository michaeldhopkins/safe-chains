use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};
use crate::policy::{self, FlagPolicy, FlagStyle};

static CODESIGN_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--deep", "--display", "--verify",
        "-R", "-d", "-v",
    ]),
    valued: WordSet::flags(&["--verbose"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_codesign(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Denied;
    }
    static CODESIGN_SAFE: WordSet = WordSet::new(&["--display", "--verify", "-d", "-v"]);
    if !tokens[1..].iter().any(|t| CODESIGN_SAFE.contains(t)) {
        return Verdict::Denied;
    }
        if policy::check(tokens, &CODESIGN_POLICY) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }

}

pub(in crate::handlers::xcode) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
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
            "https://ss64.com/mac/codesign.html",
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
