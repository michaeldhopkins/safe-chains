mod docker;
mod kubectl;

use crate::parse::{Segment, Token};

pub(crate) use docker::DOCKER;
pub(crate) use docker::PODMAN;
pub(crate) use kubectl::KUBECTL;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    DOCKER.dispatch(cmd, tokens, is_safe)
        .or_else(|| PODMAN.dispatch(cmd, tokens, is_safe))
        .or_else(|| KUBECTL.dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut doc = DOCKER.to_doc();
    doc.name = "docker / podman";
    let mut docs = vec![doc];
    docs.push(KUBECTL.to_doc());
    docs
}
