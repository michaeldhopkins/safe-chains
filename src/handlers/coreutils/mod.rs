mod awk;
mod find;
mod grep;
mod net;
pub(crate) mod sed;
mod tar;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    None
        .or_else(|| find::dispatch(cmd, tokens))
        .or_else(|| sed::dispatch(cmd, tokens))
        .or_else(|| awk::dispatch(cmd, tokens))
        .or_else(|| net::dispatch(cmd, tokens))
        .or_else(|| grep::dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(find::command_docs());
    docs.extend(awk::command_docs());
    docs.extend(net::command_docs());
    // grep/sed/tar are now `[command.behavior]` commands (engine-authoritative); their docs render
    // from the behavior (via `toml_command_docs`), with egrep/fgrep/rgrep folded into grep's single
    // entry as declared aliases. The legacy handlers stay for the dead legacy fallback + the
    // never-looser comparison, but no longer emit their own (colliding / alias-family) doc entry.
    docs
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(tar::REGISTRY);
    v.extend(sed::REGISTRY);
    v.extend(awk::REGISTRY);
    v.extend(net::registry());
    v
}
