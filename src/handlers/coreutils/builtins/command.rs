use crate::parse::{Segment, Token};

fn is_safe_command_builtin(tokens: &[Token]) -> bool {
    if tokens.len() == 2 && (tokens[1] == "--help" || tokens[1] == "--version") {
        return true;
    }
    tokens.len() >= 3
        && (tokens[1] == "-v" || tokens[1] == "-V")
}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "command" => Some(is_safe_command_builtin(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("command", "https://man7.org/linux/man-pages/man1/command.1p.html",
            "Allowed: -v, -V (check if command exists)."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Positional { cmd: "command" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        command_help: "command --help",
        command_version: "command --version",
        command_v: "command -v git",
        command_v_upper: "command -V git",
        command_v_path: "command -v /usr/bin/git",
    }

    denied! {
        command_bare_denied: "command",
        command_exec_denied: "command git status",
        command_exec_rm_denied: "command rm -rf /",
        command_only_flag_denied: "command -v",
    }
}
