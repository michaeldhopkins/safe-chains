mod docker;

use crate::parse::{Segment, Token};

pub(crate) use docker::DOCKER;
pub(crate) use docker::PODMAN;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    DOCKER.dispatch(cmd, tokens, is_safe)
        .or_else(|| PODMAN.dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut doc = DOCKER.to_doc();
    doc.name = "docker / podman";
    vec![doc]
}
