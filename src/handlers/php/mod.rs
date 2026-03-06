mod composer;
mod craft;

use crate::parse::{Segment, Token};

pub(crate) use composer::COMPOSER;
pub(crate) use craft::CRAFT;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    COMPOSER.dispatch(cmd, tokens, is_safe)
        .or_else(|| CRAFT.dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![COMPOSER.to_doc(), CRAFT.to_doc()]
}
