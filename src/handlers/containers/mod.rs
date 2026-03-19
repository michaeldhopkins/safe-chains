mod docker;
mod kubectl;
mod orbctl;

use crate::parse::Token;

pub(crate) use docker::DOCKER;
pub(crate) use docker::PODMAN;
pub(crate) use kubectl::KUBECTL;
pub(crate) use orbctl::ORBCTL;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<bool> {
    DOCKER.dispatch(cmd, tokens)
        .or_else(|| PODMAN.dispatch(cmd, tokens))
        .or_else(|| KUBECTL.dispatch(cmd, tokens))
        .or_else(|| ORBCTL.dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![DOCKER.to_doc(), PODMAN.to_doc(), KUBECTL.to_doc(), ORBCTL.to_doc()]
}
