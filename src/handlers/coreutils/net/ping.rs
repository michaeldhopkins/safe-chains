use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static PING_SAFE_STANDALONE: WordSet = WordSet::new(&[
    "-4", "-6", "-D", "-O", "-R", "-a", "-d", "-n", "-q", "-v",
]);

static PING_SAFE_VALUED: WordSet = WordSet::new(&[
    "--count", "--deadline", "--interface", "--interval",
    "--ttl",
    "-I", "-Q", "-S", "-W", "-c", "-i", "-l", "-s", "-t", "-w",
]);

fn has_count(tokens: &[Token]) -> bool {
    for t in &tokens[1..] {
        if *t == "-c" || *t == "--count" {
            return true;
        }
        let s = t.as_str();
        if s.starts_with("-c=") || s.starts_with("--count=") {
            return true;
        }
    }
    false
}

fn is_safe_ping(tokens: &[Token]) -> Verdict {
    if tokens.len() == 2 && (tokens[1] == "--help" || tokens[1] == "-h" || tokens[1] == "--version" || tokens[1] == "-V") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    if !has_count(tokens) {
        return Verdict::Denied;
    }
    let mut i = 1;
    while i < tokens.len() {
        let t = &tokens[i];
        if PING_SAFE_STANDALONE.contains(t) {
            i += 1;
            continue;
        }
        let s = t.as_str();
        if let Some(pos) = s.find('=') {
            let key = &s[..pos];
            if PING_SAFE_VALUED.contains(key) {
                i += 1;
                continue;
            }
        }
        if PING_SAFE_VALUED.contains(t) {
            i += 2;
            continue;
        }
        if !s.starts_with('-') {
            i += 1;
            continue;
        }
        return Verdict::Denied;
    }
    Verdict::Allowed(SafetyLevel::Inert)

}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "ping" => Some(is_safe_ping(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("ping",
            "https://man7.org/linux/man-pages/man8/ping.8.html",
            "Requires -c/--count to prevent infinite ping.\n\
             Flags: -4, -6, -D, -O, -R, -a, -d, -n, -q, -v.\n\
             Valued: -I, -Q, -S, -W, -c, -i, -l, -s, -t, -w, --deadline, --interface, --interval, --ttl."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "ping", valid_prefix: Some("ping -c 1") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        ping_count: "ping -c 4 localhost",
        ping_count_quiet: "ping -c 4 -q localhost",
        ping_count_interval: "ping -c 4 -i 0.5 localhost",
        ping_count_long: "ping --count=4 localhost",
        ping_ipv4: "ping -4 -c 1 localhost",
        ping_ipv6: "ping -6 -c 1 ::1",
    }

    denied! {
        ping_bare: "ping localhost",
        ping_no_count: "ping -q localhost",
        ping_flood: "ping -f localhost",
    }
}
