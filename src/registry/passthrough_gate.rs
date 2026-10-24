//! A `passthrough` is read only by a plain flag-policy sub. On a sub that dispatches any other way
//! (a handler, an executor, a guard, `first_arg`, delegation, `allow_all`) it would be silently
//! dropped, and the sub would be judged without the tail grammar its author wrote, so the
//! definition is rejected instead.
use std::collections::HashMap;

use super::policy::OwnedPolicy;
use super::types::{DispatchKind, TomlSub};

pub(super) fn build_sub_kind(parent: &str, toml: TomlSub, handler_policies: &HashMap<String, OwnedPolicy>) -> Result<DispatchKind, String> {
    let has_passthrough = toml.passthrough.is_some();
    let sub_name = toml.name.clone();
    let allow_all = toml.allow_all.unwrap_or(false);
    let kind = super::build::build_sub_kind_unchecked(parent, toml, handler_policies)?;
    if has_passthrough && (allow_all || !matches!(kind, DispatchKind::Policy { .. })) {
        return Err(format!(
            "command '{parent}' sub `{sub_name}` declares a `passthrough` but dispatches as something other than a plain flag policy \
             (handler, executor, guard, require_any, write_flags, first_arg, delegation or allow_all). Only a plain policy sub \
             applies a passthrough; on any other sub it would be silently dropped."
        ));
    }
    Ok(kind)
}
