mod composer;
mod craft;

use crate::parse::Token;

pub(crate) use composer::COMPOSER;
pub(crate) use craft::CRAFT;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<bool> {
    COMPOSER.dispatch(cmd, tokens)
        .or_else(|| CRAFT.dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![COMPOSER.to_doc(), CRAFT.to_doc()]
}
