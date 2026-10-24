//! A sub that hands the words after a separator to the classifier as a command of their own
//! (`mise exec node@20 -- cargo fmt`). The words before it are the sub's own: operands, judged by
//! its refused operands, and flags, approved only when the sub lists them, since one it does not
//! list may run a command of its own or move the inner one (`mise exec -c`, `-C`).

use crate::parse::Token;
use crate::verdict::Verdict;

#[derive(Debug, Clone)]
pub(crate) struct DelegateAfter {
    pub(crate) separator: String,
    pub(crate) standalone: Vec<String>,
    pub(crate) valued: Vec<String>,
}

impl DelegateAfter {
    pub(crate) fn new(separator: String, standalone: Vec<String>, valued: Vec<String>) -> Self {
        DelegateAfter { separator, standalone, valued }
    }

    pub(super) fn verdict(&self, tokens: &[Token]) -> Verdict {
        let Some(pos) = tokens[1..].iter().position(|t| t == self.separator.as_str()) else {
            return Verdict::Denied;
        };
        let inner_start = pos + 2;
        if !self.own_flags_listed(&tokens[1..=pos]) || inner_start >= tokens.len() {
            return Verdict::Denied;
        }
        let inner = shell_words::join(tokens[inner_start..].iter().map(|t| t.as_str()));
        crate::command_verdict(&inner)
    }

    /// Whether every flag among `args` is a listed standalone flag, or a listed valued one with its
    /// value glued on with `=` or in the next word. A next word that reads as a flag is not taken for
    /// the value, so `--jobs -c` cannot hide the `-c`.
    fn own_flags_listed(&self, args: &[Token]) -> bool {
        let mut i = 0;
        while i < args.len() {
            let t = args[i].as_str();
            let glued = self.valued.iter().any(|f| t.strip_prefix(f.as_str()).is_some_and(|rest| rest.starts_with('=')));
            if !t.starts_with('-') || glued || self.standalone.iter().any(|f| t == f) {
                i += 1;
            } else if self.valued.iter().any(|f| t == f) && args.get(i + 1).is_some_and(|v| !v.starts_with('-')) {
                i += 2;
            } else {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::DelegateAfter;
    use crate::parse::Token;
    use proptest::prelude::*;

    fn delegate() -> DelegateAfter {
        DelegateAfter::new("--".into(), vec!["-q".into(), "--raw".into()], vec!["-j".into(), "--jobs".into()])
    }

    fn words(spellings: &[u8], value: &str) -> Vec<String> {
        let mut out = Vec::new();
        for spelling in spellings {
            match spelling {
                0 => out.push("-q".to_string()),
                1 => out.push("--raw".to_string()),
                2 => out.extend(["-j".to_string(), value.to_string()]),
                3 => out.push(format!("--jobs={value}")),
                _ => out.push(format!("node@{value}")),
            }
        }
        out
    }

    fn listed(words: &[String]) -> bool {
        let tokens: Vec<Token> = words.iter().map(|w| Token::from_test(w)).collect();
        delegate().own_flags_listed(&tokens)
    }

    proptest! {
        /// Listed flags, with their values in either spelling, and operands pass in any order; one
        /// unlisted flag anywhere among them, or a valued flag whose next word is a flag or missing,
        /// fails closed.
        #[test]
        fn only_listed_flags_pass_before_the_separator(
            spellings in prop::collection::vec(0u8..5, 0..6),
            value in "[a-z0-9]{1,4}",
            unlisted in prop::sample::select(vec!["-c", "--command=x", "-C", "--cd", "-Cx", "-E", "--env=x", "-j4", "-x"]),
            at in any::<prop::sample::Index>(),
        ) {
            let mut all = words(&spellings, &value);
            prop_assert!(listed(&all), "{:?}", all);
            let mut with_unlisted = all.clone();
            with_unlisted.insert(at.index(all.len() + 1), unlisted.to_string());
            prop_assert!(!listed(&with_unlisted), "{:?}", with_unlisted);
            all.push("--jobs".into());
            prop_assert!(!listed(&all), "{:?}", all);
            all.push("-c".into());
            prop_assert!(!listed(&all), "{:?}", all);
        }
    }
}
