use crate::parse::{Segment, Token};

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, _tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "safe-chains" => Some(true),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("safe-chains",
            "Any arguments allowed (safe-chains is this tool)."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Positional { cmd: "safe-chains" },
];
