use crate::parse::Token;
use crate::policy::check_flags;

use super::types::OwnedPolicy;

pub(super) fn check_owned(tokens: &[Token], policy: &OwnedPolicy) -> bool {
    check_flags(
        tokens,
        policy.standalone.as_slice(),
        policy.valued.as_slice(),
        policy.bare,
        policy.max_positional,
        policy.flag_style,
    )
}
