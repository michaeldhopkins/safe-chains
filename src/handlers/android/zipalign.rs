use crate::parse::{Segment, Token};

pub fn is_safe_zipalign(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let mut has_c = false;
    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        match t {
            "-c" => { has_c = true; i += 1; }
            "-v" | "-p" => { i += 1; }
            _ if !t.starts_with('-') => { i += 1; }
            _ => return false,
        }
    }
    has_c
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    if cmd == "zipalign" {
        Some(is_safe_zipalign(tokens))
    } else {
        None
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("zipalign",
            "https://developer.android.com/tools/zipalign",
            "Check mode only (requires -c). Flags: -p, -v."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::android) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "zipalign", valid_prefix: Some("zipalign -c") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        zipalign_check: "zipalign -c 4 app.apk",
        zipalign_check_verbose: "zipalign -c -v 4 app.apk",
        zipalign_check_page: "zipalign -c -p 4 app.apk",
    }

    denied! {
        zipalign_bare_denied: "zipalign",
        zipalign_align_denied: "zipalign 4 input.apk output.apk",
        zipalign_force_denied: "zipalign -f 4 input.apk output.apk",
        zipalign_unknown_denied: "zipalign -c --unknown 4 app.apk",
    }
}
