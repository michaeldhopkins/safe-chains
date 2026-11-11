//! A `$` the shell does not expand, in a word that sets the command search path.
//!
//! The search-path rule trusts an element spelled `$PATH`, because the shell replaces it with the
//! folders already searched. Quoted or escaped (`'$PATH'`, `\$PATH`, `$'$PATH'`), the shell keeps
//! the five characters, and `PATH='$PATH' ls` searches a folder named `$PATH` below the working
//! directory. The value the rule reads is the same text either way, so the quoting is read here.

use super::{SimpleCmd, Word, WordPart};
use crate::verdict::{SafetyLevel, Verdict};

pub(super) fn verdict(cmd: &SimpleCmd) -> Verdict {
    let assigned = cmd.env.iter().any(|(name, value)| sets_search(name) && keeps_a_dollar(value));
    let worded = cmd.words.iter().any(keeps_a_dollar) && cmd.words.iter().any(|w| names_search(&w.eval()));
    if assigned || worded { Verdict::Denied } else { Verdict::Allowed(SafetyLevel::Inert) }
}

fn sets_search(name: &str) -> bool {
    crate::envvars::names_command_search(name.trim_end_matches('+'))
}

fn names_search(text: &str) -> bool {
    sets_search(text.split_once('=').map_or(text, |(name, _)| name))
}

fn keeps_a_dollar(word: &Word) -> bool {
    word.0.iter().any(|part| match part {
        WordPart::SQuote(s) => s.contains('$'),
        WordPart::AnsiC(raw) => super::ansi_c::decode(raw).contains('$'),
        WordPart::Escape(c) => *c == '$',
        WordPart::DQuote(inner) => keeps_a_dollar(inner),
        _ => false,
    })
}

#[cfg(test)]
#[path = "search_quote_tests.rs"]
mod tests;
