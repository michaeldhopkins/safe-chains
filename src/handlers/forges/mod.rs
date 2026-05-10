// Hand-written instead of `handler_module!(gh, glab)` because both gh
// and glab are dispatched through the TOML registry now (handler = "gh"
// and handler = "glab" in their TOMLs), but `is_safe_gh_api` is still
// needed at module path `forges::gh::is_safe_gh_api` for the
// custom_sub_handlers map and for glab's `api` sub delegation.
pub(crate) mod gh;
pub(crate) mod glab;

pub(crate) fn dispatch(cmd: &str, tokens: &[crate::parse::Token]) -> Option<crate::verdict::Verdict> {
    None
        .or_else(|| gh::dispatch(cmd, tokens))
        .or_else(|| glab::dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(gh::command_docs());
    docs.extend(glab::command_docs());
    docs
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(gh::REGISTRY);
    v.extend(glab::REGISTRY);
    v
}
