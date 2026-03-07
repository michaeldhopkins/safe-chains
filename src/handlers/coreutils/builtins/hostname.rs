use crate::parse::{Segment, Token, WordSet};

static HOSTNAME_DISPLAY: WordSet = WordSet::new(&["-A", "-I", "-d", "-f", "-i", "-s"]);

fn is_safe_hostname(tokens: &[Token]) -> bool {
    if tokens.len() == 1 {
        return true;
    }
    if tokens.len() == 2 && matches!(tokens[1].as_str(), "--help" | "-h" | "--version" | "-V") {
        return true;
    }
    tokens[1..].iter().all(|t| HOSTNAME_DISPLAY.contains(t))
}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "hostname" => Some(is_safe_hostname(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::wordset("hostname", "https://man7.org/linux/man-pages/man1/hostname.1.html", &HOSTNAME_DISPLAY),
    ]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "hostname", valid_prefix: None },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        hostname_help: "hostname --help",
        hostname_version: "hostname --version",
        hostname_bare: "hostname",
        hostname_fqdn: "hostname -f",
        hostname_short: "hostname -s",
        hostname_domain: "hostname -d",
        hostname_ip: "hostname -i",
        hostname_all_ip: "hostname -I",
        hostname_all_addr: "hostname -A",
    }

    denied! {
        hostname_set_denied: "hostname evil",
        hostname_set_fqdn_denied: "hostname new.example.com",
        hostname_flag_with_name_denied: "hostname -f evil",
    }
}
