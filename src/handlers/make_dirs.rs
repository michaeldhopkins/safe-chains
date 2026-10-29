//! The folder a make line runs in, when its options move it more than once.
//!
//! GNU make applies each `-C` in turn, each relative to the folder the last one reached, and reads a
//! relative `-f` from the folder the last `-C` left it in. Each option judged alone names a folder
//! the line never reaches (`cd src && make -n -C .. -C ..` runs the makefile two levels up).

use crate::parse::Token;

/// Whether a make line moves its folder more than once, or moves it and names a relative makefile.
pub(crate) fn moves_its_folder_out_of_sight(tokens: &[Token]) -> bool {
    let words: Vec<&str> = tokens.iter().skip(1).map(Token::as_str).collect();
    let mut moves = 0usize;
    let mut relative_makefile = false;
    let mut at = 0;
    while at < words.len() {
        let word = words[at];
        at += 1;
        if let Some(long) = word.strip_prefix("--") {
            let (name, inline) = match long.split_once('=') {
                Some((name, value)) => (name, Some(value)),
                None => (long, None),
            };
            let value = match name {
                "directory" | "file" | "makefile" => inline.or_else(|| words.get(at).copied()),
                _ => continue,
            };
            if inline.is_none() {
                at += 1;
            }
            if name == "directory" {
                moves += 1;
            } else {
                relative_makefile |= value.is_none_or(|v| !v.starts_with('/'));
            }
            continue;
        }
        let Some(cluster) = word.strip_prefix('-').filter(|c| !c.is_empty()) else {
            continue;
        };
        for (index, letter) in cluster.char_indices() {
            if !matches!(letter, 'C' | 'f' | 'j' | 'I' | 'l' | 'o' | 'W' | 'E') {
                continue;
            }
            let rest = &cluster[index + letter.len_utf8()..];
            let value = if rest.is_empty() && letter != 'j' {
                at += 1;
                words.get(at - 1).copied()
            } else {
                Some(rest)
            };
            match letter {
                'C' => moves += 1,
                'f' => relative_makefile |= value.is_none_or(|v| !v.starts_with('/')),
                _ => {}
            }
            break;
        }
    }
    moves > 1 || (moves == 1 && relative_makefile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn refused(line: &str) -> bool {
        let tokens: Vec<Token> = line.split_whitespace().map(Token::from_test).collect();
        moves_its_folder_out_of_sight(&tokens)
    }

    #[test]
    fn one_move_or_a_makefile_alone_is_left_to_the_path_gate() {
        for line in [
            "make -n", "make -n -C src", "make -nC src", "make -nCsrc", "make --directory=src -n", "make -n -f Makefile",
            "make -n -f a.mk -f b.mk", "make -j4 -n -C src", "make -n -C src -f /abs/Makefile", "make -n -j -C src",
        ] {
            assert!(!refused(line), "{line}");
        }
    }

    fn spell_directory(form: usize, dir: &str) -> String {
        match form {
            0 => format!("-C {dir}"),
            1 => format!("-C{dir}"),
            2 => format!("-nC {dir}"),
            3 => format!("--directory={dir}"),
            _ => format!("--directory {dir}"),
        }
    }

    proptest! {
        /// Two folder moves, in any spelling of either and with anything between them, are refused.
        #[test]
        fn a_second_move_is_refused(
            first in 0usize..5,
            second in 0usize..5,
            a in "[a-z.]{1,4}",
            b in "[a-z.]{1,4}",
            between in prop::sample::select(vec!["", "-n", "all", "-j4", "-q"]),
        ) {
            let line = format!("make -n {} {between} {}", spell_directory(first, &a), spell_directory(second, &b));
            prop_assert!(refused(&line), "{}", line);
        }

        /// A move with a relative makefile, in any spelling of either, is refused; an absolute one is not.
        #[test]
        fn a_move_with_a_relative_makefile_is_refused(
            form in 0usize..5,
            dir in "[a-z.]{1,4}",
            makefile in "[a-z.][a-z./]{0,8}",
            spelling in prop::sample::select(vec!["-f {}", "-f{}", "-nf {}", "--file={}", "--file {}", "--makefile={}"]),
        ) {
            let line = format!("make -n {} {}", spell_directory(form, &dir), spelling.replace("{}", &makefile));
            prop_assert!(refused(&line), "{}", line);
            let absolute = format!("make -n {} {}", spell_directory(form, &dir), spelling.replace("{}", &format!("/{makefile}")));
            prop_assert!(!refused(&absolute), "{}", absolute);
        }
    }
}
