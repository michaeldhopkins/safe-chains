use crate::parse::{Segment, Token, WordSet};

static FD_EXEC_LONG: WordSet = WordSet::new(&["--exec", "--exec-batch"]);

pub(in crate::handlers::coreutils) fn is_safe_fd(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    for t in &tokens[1..] {
        if FD_EXEC_LONG.contains(t) {
            return false;
        }
        if t.starts_with('-')
            && !t.starts_with("--")
            && t.as_bytes()[1..].iter().any(|&b| b == b'x' || b == b'X')
        {
            return false;
        }
    }
    true
}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "fd" => Some(is_safe_fd(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("fd",
            "Safe unless --exec/-x or --exec-batch/-X flags (execute arbitrary commands)."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Positional { cmd: "fd" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        fd_pattern: "fd pattern",
        fd_hidden: "fd -H pattern",
        fd_type: "fd -t f pattern",
        fd_extension: "fd -e rs pattern",
        fd_glob: "fd -g '*.rs'",
        fd_follow: "fd -L pattern",
        fd_absolute: "fd -a pattern",
        fd_color: "fd --color auto pattern",
        fd_max_depth: "fd --max-depth 3 pattern",
    }

    denied! {
        fd_exec_denied: "fd pattern --exec rm",
        fd_exec_short_denied: "fd pattern -x rm",
        fd_exec_batch_denied: "fd -t f pattern --exec-batch rm",
        fd_exec_batch_short_denied: "fd pattern -X rm",
        fd_exec_combined_denied: "fd -xH pattern",
        fd_exec_batch_combined_denied: "fd -HX pattern",
        fd_bare_denied: "fd",
    }
}
