// gh and glab dispatch through the TOML registry (handler = "gh" /
// handler = "glab" in their TOMLs). `forges::gh::is_safe_gh_api` is
// still needed at this module path for the custom_sub_handlers map
// and for glab's `api` sub delegation.
pub(crate) mod gh;
pub(crate) mod glab;

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(gh::REGISTRY);
    v.extend(glab::REGISTRY);
    v
}
