//! The credential-name check of a `first_arg` glob, out of `registry/dispatch.rs`.

use crate::parse::Token;

/// Whether any argument from the glob's first one on names secret material.
///
/// Every positional is asked, and each comma-separated item in it: kubectl reads several kinds in
/// one call (`get pods,secrets`, `get pod/a secret/b`), so a credential kind in second place reads
/// as much as one in first. The value of a listed valued flag is not an argument (`-n secrets` is a
/// namespace). Matched CASE-INSENSITIVELY: kubectl resource kinds are case-insensitive (`get
/// Secret`/`SECRET` reads secrets), so a case-variant must not slip past the deny into the `*`
/// allow. The allow-glob stays case-sensitive — a case-insensitive ALLOW would be fail-open.
pub(super) fn names_a_credential(arguments: &[Token], credential: &[String], valued: &[&[String]]) -> bool {
    if credential.is_empty() {
        return false;
    }
    let is_valued = |word: &str| valued.iter().any(|list| list.iter().any(|f| f == word));
    let matches = |item: &str| {
        credential.iter().any(|p| match p.strip_suffix('*') {
            Some(prefix) => item.len() >= prefix.len() && item.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes()),
            None => item.eq_ignore_ascii_case(p),
        })
    };
    let mut words = arguments.iter().map(Token::as_str);
    while let Some(word) = words.next() {
        if word.starts_with('-') && word != "-" {
            if is_valued(word) {
                words.next();
            }
            continue;
        }
        if word.split(',').any(&matches) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::names_a_credential;
    use crate::parse::Token;
    use proptest::prelude::*;

    proptest! {
        /// A credential name is found wherever it stands among the arguments, alone or as one
        /// item of a comma list, in any case; the same words without it, or with it only as a
        /// listed flag's value, are not a credential read.
        #[test]
        fn a_credential_name_is_found_in_any_position(
            others in prop::collection::vec("[a-r][a-z]{1,8}", 1..5),
            at in 0usize..5,
            comma in any::<bool>(),
            upper in any::<bool>(),
        ) {
            let credential = vec!["secret".to_string(), "secrets/*".to_string()];
            let valued = vec!["-n".to_string()];
            let name = if upper { "SECRET" } else { "secret" };
            let at = at.min(others.len());
            let mut items = others.clone();
            items.insert(at, name.to_string());
            let words: Vec<String> = if comma { vec![items.join(",")] } else { items };
            let tokens: Vec<Token> = words.iter().map(|w| Token::from_test(w)).collect();
            prop_assert!(names_a_credential(&tokens, &credential, &[&valued]), "{:?}", words);
            let plain: Vec<Token> = others.iter().map(|w| Token::from_test(w)).collect();
            prop_assert!(!names_a_credential(&plain, &credential, &[&valued]), "{:?}", others);
            let mut as_value: Vec<Token> = plain.clone();
            as_value.push(Token::from_test("-n"));
            as_value.push(Token::from_test(name));
            prop_assert!(!names_a_credential(&as_value, &credential, &[&valued]), "{:?} -n {}", others, name);
        }
    }
}
