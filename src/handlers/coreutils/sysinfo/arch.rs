use crate::parse::{Segment, Token};

fn is_safe_arch(tokens: &[Token]) -> bool {
    tokens.len() == 1
}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "arch" => Some(is_safe_arch(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("arch", "Bare invocation allowed."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "arch", valid_prefix: None },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        arch_bare: "arch",
        arch_help: "arch --help",
        arch_version: "arch --version",
    }

    denied! {
        arch_exec_denied: "arch -x86_64 rm -rf /",
        arch_flag_denied: "arch -arm64 echo hello",
        arch_any_flag_denied: "arch -x86_64",
    }
}
