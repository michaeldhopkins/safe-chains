//! `git mv`: the operands of a move git performs on disk.

use crate::parse::Token;

/// `git mv SOURCE… DEST` moves each source to the destination on disk, so it is judged as `mv` is:
/// the operands go to the engine as an `mv` of the same words, which places each one (spelled and
/// physical forms, links acted on as themselves, trust roots, the project's edge) and takes the
/// stricter of the source and the destination. Git's own flags choose how it reports or whether it
/// skips an error, never where a path lands, so none is passed on. Fewer than two operands is a
/// usage error git refuses before moving anything (`--help`, a ref named `mv`).
pub(in crate::pathgate) fn git_mv(tokens: &[Token]) -> bool {
    let mut after_options = false;
    let mut operands: Vec<Token> = Vec::new();
    for token in tokens.iter().skip(1) {
        let word = token.as_str();
        if after_options || word == "-" || !word.starts_with('-') {
            operands.push(token.clone());
        } else if word == "--" {
            after_options = true;
        }
    }
    if operands.len() < 2 {
        return false;
    }
    let mut moved = vec![Token::from_raw("mv".to_string()), Token::from_raw("--".to_string())];
    moved.extend(operands);
    crate::engine::resolve::resolve(&moved).is_none_or(|profile| !crate::engine::bridge::project(&profile).is_allowed())
}

#[cfg(test)]
mod tests {
    use super::git_mv;
    use crate::parse::Token;
    use proptest::prelude::*;

    fn words(parts: &[String]) -> Vec<Token> {
        parts.iter().map(|p| Token::from_raw(p.clone())).collect()
    }

    proptest! {
        #[test]
        fn git_flags_never_change_where_the_operands_land(
            source in "[a-z./~.]{1,12}",
            dest in "[a-z./~.]{1,12}",
            flags in proptest::sample::subsequence(vec!["-f", "-n", "-v", "-k", "--force", "--dry-run"], 0..4),
        ) {
            let bare = git_mv(&words(&["mv".into(), source.clone(), dest.clone()]));
            let mut with_flags = vec!["mv".to_string()];
            with_flags.extend(flags.iter().map(|f| f.to_string()));
            with_flags.extend([source, dest]);
            prop_assert_eq!(super::super::dispatch("git_mv", &words(&with_flags)), bare);
            prop_assert!(!super::super::dispatch("git_mv", &words(&["mv".into(), "src/a.rs".into(), "src/b.rs".into()])));
            prop_assert!(super::super::dispatch("git_mv", &words(&["mv".into(), "-".into(), ".envrc".into()])));
            prop_assert!(super::super::dispatch("git_mv", &words(&["mv".into(), "--".into(), "-f".into(), ".envrc".into()])));
            prop_assert!(super::super::dispatch("git_mv", &words(&["mv".into(), "src/a.rs".into(), "src/b.rs".into(), ".envrc".into()])));
        }

        #[test]
        fn fewer_than_two_operands_is_never_judged_as_a_move(
            only in "[a-z./~.]{1,12}",
            flags in proptest::sample::subsequence(vec!["-f", "-n", "-v", "-k", "--force", "--dry-run"], 0..4),
        ) {
            let mut zero = vec!["mv".to_string()];
            zero.extend(flags.iter().map(|f| f.to_string()));
            prop_assert!(!git_mv(&words(&zero)));
            let mut one = zero;
            one.push(only);
            prop_assert!(!git_mv(&words(&one)));
            prop_assert!(!git_mv(&words(&["mv".into(), "-f".into(), "--".into(), ".envrc".into()])));
        }
    }
}
