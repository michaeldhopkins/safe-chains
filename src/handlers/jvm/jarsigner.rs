use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static VERIFY_STANDALONE: WordSet = WordSet::new(&[
    "-certs", "-strict", "-verbose",
]);

pub fn is_safe_jarsigner(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Denied;
    }
    if tokens.len() == 2 && (tokens[1] == "-help" || tokens[1] == "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let mut has_verify = false;
    let mut i = 1;
    while i < tokens.len() {
        let t = &tokens[i];
        if t == "-verify" {
            has_verify = true;
            i += 1;
            continue;
        }
        if !t.starts_with('-') {
            i += 1;
            continue;
        }
        if VERIFY_STANDALONE.contains(t) {
            i += 1;
            continue;
        }
        return Verdict::Denied;
    }
        if has_verify { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }

}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    if cmd == "jarsigner" {
        Some(is_safe_jarsigner(tokens))
    } else {
        None
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("jarsigner",
            "https://docs.oracle.com/en/java/javase/21/docs/specs/man/jarsigner.html",
            "Verify mode only (requires -verify). Flags: -certs, -strict, -verbose."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::jvm) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "jarsigner", valid_prefix: Some("jarsigner -verify") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        jarsigner_verify: "jarsigner -verify app.jar",
        jarsigner_verify_verbose: "jarsigner -verify -verbose app.jar",
        jarsigner_verify_certs: "jarsigner -verify -certs app.jar",
        jarsigner_verify_strict: "jarsigner -verify -strict app.jar",
        jarsigner_verify_all: "jarsigner -verify -verbose -certs -strict app.apk",
    }

    denied! {
        jarsigner_bare_denied: "jarsigner",
        jarsigner_sign_denied: "jarsigner -keystore debug.keystore app.jar alias",
        jarsigner_no_verify_denied: "jarsigner app.jar",
        jarsigner_verify_unknown_denied: "jarsigner -verify --unknown app.jar",
    }
}
