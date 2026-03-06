use crate::parse::{Segment, Token};

fn is_safe_nslookup(tokens: &[Token]) -> bool {
    for t in &tokens[1..] {
        let s = t.as_str();
        if !s.starts_with('-') {
            continue;
        }
        if s == "-debug" || s == "-nodebug" || s == "-d2" {
            continue;
        }
        if s.starts_with("-type=")
            || s.starts_with("-query=")
            || s.starts_with("-port=")
            || s.starts_with("-timeout=")
            || s.starts_with("-retry=")
            || s.starts_with("-class=")
            || s.starts_with("-domain=")
            || s.starts_with("-querytype=")
        {
            continue;
        }
        return false;
    }
    true
}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "nslookup" => Some(is_safe_nslookup(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("nslookup",
            "Allowed: positional args, -debug, -nodebug, -d2, and valued options (-type=, -query=, -port=, -timeout=, -retry=, -class=, -domain=, -querytype=)."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "nslookup", valid_prefix: Some("nslookup example.com") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        nslookup_domain: "nslookup example.com",
        nslookup_server: "nslookup example.com 8.8.8.8",
        nslookup_type: "nslookup -type=MX example.com",
    }
}
