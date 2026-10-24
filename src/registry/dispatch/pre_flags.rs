//! The flags in front of a branching command's first positional, out of `registry/dispatch.rs`.

use crate::parse::Token;
use crate::policy::FlagSet;

/// Whether `s` is a rustup toolchain selector (`+nightly`, `+1.90.0`, `+nightly-2026-01-01`).
///
/// A POSITIVE shape test, so an unrecognized spelling is simply not a selector and dispatch
/// proceeds without stripping it (which then denies on the unknown token). The selector names an
/// ALREADY-INSTALLED toolchain — rustup resolves `+name` through its own toolchain list and cannot
/// be pointed at an arbitrary directory — so the shape is restricted to what a toolchain name can
/// be. `/`, `~`, `$`, `.` and `..` are excluded by construction rather than by exclusion: only
/// alphanumerics and `._-` are admitted, and a name must start alphanumeric so `+..` cannot form.
fn is_toolchain_selector(s: &str) -> bool {
    let Some(name) = s.strip_prefix('+') else { return false };
    !name.is_empty()
        && name.starts_with(|c: char| c.is_ascii_alphanumeric())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

pub(super) fn skip_pre_flags(
    tokens: &[Token],
    pre_standalone: &[String],
    pre_valued: &[String],
    toolchain_selector: bool,
    start: usize,
) -> usize {
    let mut i = start;
    // At most ONE, and only in front. `cargo +nightly build` selects which toolchain runs `build`;
    // it does not change what `build` does, which is why the sub dispatch below is reached
    // unchanged. Bounded to a single leading token so a `+`-prefixed OPERAND deeper in the line is
    // never silently eaten.
    if toolchain_selector
        && let Some(t) = tokens.get(i)
        && is_toolchain_selector(t.as_str())
    {
        i += 1;
    }
    while i < tokens.len() {
        let t = &tokens[i];
        let s = t.as_str();
        if !s.starts_with('-') {
            break;
        }
        if pre_valued.contains_flag(s) && pre_standalone.contains_flag(s) {
            break;
        }
        if pre_valued.contains_flag(s) {
            i += 2;
            continue;
        }
        if let Some((flag, _)) = s.split_once('=')
            && pre_valued.contains_flag(flag)
        {
            i += 1;
            continue;
        }
        if pre_standalone.contains_flag(s) {
            i += 1;
            continue;
        }
        // A listed valued short with its value glued on (`-oyaml`, `-nprod`), as getopt and pflag
        // read it: the rest of the word is the value, not more flags.
        if s.len() > 2
            && !s.starts_with("--")
            && let Some(short) = s.get(..2)
            && pre_valued.contains_flag(short)
        {
            i += 1;
            continue;
        }
        // POSIX-style short-flag cluster (`-vv`, `-vy`): every byte after
        // the dash must be a known standalone short. Mirrors the same
        // logic in policy::check_flags for non-wrapper subs.
        let bytes = s.as_bytes();
        if bytes.len() > 2 && bytes[1] != b'-' && bytes[1..].iter().all(|&b| pre_standalone.contains_short(b)) {
            i += 1;
            continue;
        }
        break;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::skip_pre_flags;
    use crate::parse::Token;
    use proptest::prelude::*;

    proptest! {
        /// Each listed flag in front is skipped with its value, whether the value is the next word,
        /// follows `=`, or is glued to a valued short; the first positional or unlisted flag stops it.
        #[test]
        fn the_listed_flags_in_front_are_skipped_with_their_values(
            spellings in prop::collection::vec(0u8..5, 0..5),
            value in "[a-z0-9=]{1,6}",
            stop in prop::sample::select(vec!["pods", "-x", "-xvalue", "--unlisted"]),
        ) {
            let standalone = vec!["-v".to_string(), "--verbose".to_string()];
            let valued = vec!["-o".to_string(), "--output".to_string()];
            let mut words: Vec<String> = Vec::new();
            for spelling in &spellings {
                match spelling {
                    0 => words.push("-v".into()),
                    1 => words.extend(["-o".to_string(), value.clone()]),
                    2 => words.push(format!("-o{value}")),
                    3 => words.push(format!("--output={value}")),
                    _ => words.extend(["--output".to_string(), value.clone()]),
                }
            }
            let flags_end = words.len() + 1;
            words.insert(0, "get".into());
            words.push(stop.to_string());
            words.push("rest".into());
            let tokens: Vec<Token> = words.iter().map(|w| Token::from_test(w)).collect();
            prop_assert_eq!(skip_pre_flags(&tokens, &standalone, &valued, false, 1), flags_end, "{:?}", words);
        }

        /// A bare flag listed both standalone and valued stops the skip at itself, so neither the
        /// word after it nor the one after that is taken for the subcommand.
        #[test]
        fn a_flag_listed_both_ways_stops_the_skip_at_itself(
            before in prop::collection::vec(prop::sample::select(vec!["-v", "--verbose"]), 0..4),
            dual in prop::sample::select(vec!["-l", "--lock"]),
        ) {
            let standalone = vec!["-v".to_string(), "--verbose".to_string(), "-l".to_string(), "--lock".to_string()];
            let valued = vec!["-l".to_string(), "--lock".to_string()];
            let mut words: Vec<String> = vec!["borg".into()];
            words.extend(before.iter().map(|w| w.to_string()));
            let at = words.len();
            words.extend([dual.to_string(), "delete".into(), "info".into()]);
            let tokens: Vec<Token> = words.iter().map(|w| Token::from_test(w)).collect();
            prop_assert_eq!(skip_pre_flags(&tokens, &standalone, &valued, false, 1), at, "{:?}", words);
            let glued: Vec<Token> = ["borg", &format!("{dual}=5"), "info"].iter().map(|w| Token::from_test(w)).collect();
            prop_assert_eq!(skip_pre_flags(&glued, &standalone, &valued, false, 1), 2);
        }
    }
}
