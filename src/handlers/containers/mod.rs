mod docker;
mod orbctl;
mod qemu_img;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) use docker::DOCKER;
pub(crate) use docker::PODMAN;
pub(crate) use orbctl::ORBCTL;
pub(crate) use qemu_img::QEMU_IMG;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    DOCKER.dispatch(cmd, tokens)
        .or_else(|| PODMAN.dispatch(cmd, tokens))
        .or_else(|| ORBCTL.dispatch(cmd, tokens))
        .or_else(|| QEMU_IMG.dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![DOCKER.to_doc(), PODMAN.to_doc(), ORBCTL.to_doc(), QEMU_IMG.to_doc()]
}
