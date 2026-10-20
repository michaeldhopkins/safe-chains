use crate::parse::Token;

const ID_BITS: u32 = 0o6000;

pub(crate) fn sets_an_id_bit(tokens: &[Token]) -> bool {
    let mut i = 1;
    while i < tokens.len() {
        let word = tokens[i].as_str();
        if word == "--reference" || word.starts_with("--reference=") {
            return true;
        }
        if word == "--" {
            return tokens.get(i + 1).is_some_and(|mode| mode_sets_an_id_bit(mode.as_str()));
        }
        if !word.starts_with('-') || is_mode_spelled_as_a_flag(word) {
            return mode_sets_an_id_bit(word);
        }
        i += 1;
    }
    false
}

fn is_mode_spelled_as_a_flag(word: &str) -> bool {
    word.len() > 1 && word[1..].chars().all(|c| matches!(c, 'r' | 'w' | 'x' | 'X' | 's' | 't'))
}

fn mode_sets_an_id_bit(mode: &str) -> bool {
    if !mode
        .chars()
        .all(|c| matches!(c, 'u' | 'g' | 'o' | 'a' | '+' | '-' | '=' | 'r' | 'w' | 'x' | 'X' | 's' | 't' | ',' | '0'..='7'))
    {
        return true;
    }
    mode.split(',').any(clause_sets_an_id_bit)
}

fn clause_sets_an_id_bit(clause: &str) -> bool {
    if !clause.is_empty() && clause.chars().all(|c| c.is_ascii_digit()) {
        return octal_has_an_id_bit(clause);
    }
    let mut adds = false;
    let mut digits = String::new();
    for c in clause.chars() {
        match c {
            '+' | '=' | '-' => {
                if adds && octal_has_an_id_bit(&digits) {
                    return true;
                }
                digits.clear();
                adds = c != '-';
            }
            '0'..='7' => digits.push(c),
            's' if adds => return true,
            _ => {}
        }
    }
    adds && octal_has_an_id_bit(&digits)
}

fn octal_has_an_id_bit(digits: &str) -> bool {
    if digits.is_empty() {
        return false;
    }
    u32::from_str_radix(digits, 8).map_or(true, |value| value & ID_BITS != 0)
}

#[cfg(test)]
mod tests {
    use super::mode_sets_an_id_bit;
    use crate::is_safe_command;
    use proptest::prelude::*;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        chmod_exec: "chmod +x scripts/build.sh",
        chmod_octal: "chmod 644 README.md",
        chmod_octal_four_digits: "chmod 0755 bin/tool",
        chmod_sticky: "chmod +t tmpdir",
        chmod_drop_setuid: "chmod u-s bin/tool",
        chmod_symbolic_list: "chmod -R u+rwX,go-w src",
    }

    denied! {
        chmod_plus_s: "chmod +s x",
        chmod_u_plus_s: "chmod u+s x",
        chmod_g_plus_s: "chmod g+s x",
        chmod_a_equals_rwxs: "chmod a=rwxs x",
        chmod_u_equals_s: "chmod u=rwxs x",
        chmod_list_with_s: "chmod go-w,u+s x",
        chmod_octal_setuid: "chmod 4755 x",
        chmod_octal_setgid: "chmod 2755 x",
        chmod_octal_both: "chmod 6755 x",
        chmod_octal_leading_zero: "chmod 04755 x",
        chmod_octal_many_digits: "chmod 0004755 x",
        chmod_plus_octal: "chmod +4000 x",
        chmod_equals_octal: "chmod =2755 x",
        chmod_minus_then_plus: "chmod u-x+s x",
        chmod_gnu_alias: "gchmod +s x",
        chmod_absolute: "/bin/chmod +s x",
        chmod_quoted: "chmod '+s' x",
        chmod_after_separator: "chmod -- +s x",
        chmod_verbose: "chmod -v u+s x",
        chmod_recursive: "chmod -R g+s dir",
        chmod_variable_mode: "chmod \"$MODE\" x",
        chmod_env_prefix: "FOO=1 chmod +s x",
        chmod_subshell: "(chmod +s x)",
        chmod_in_find: "find . -name x -exec chmod +s {} \\;",
        chmod_via_xargs: "echo x | xargs chmod +s",
        chmod_reference_to_a_setuid_file: "chmod --reference=/bin/su x",
        chmod_reference_spaced: "chmod --reference /bin/su x",
    }

    proptest! {
        #[test]
        fn an_octal_mode_sets_an_id_bit_exactly_when_its_value_has_one(value in 0u32..0o10000, pad in 0usize..3) {
            let mode = format!("{}{value:o}", "0".repeat(pad));
            prop_assert_eq!(mode_sets_an_id_bit(&mode), value & 0o6000 != 0, "{}", mode);
        }

        #[test]
        fn adding_s_for_any_class_sets_an_id_bit(
            who in "[ugoa]{0,3}",
            op in prop_oneof![Just('+'), Just('=')],
            before in "[rwxXt]{0,3}",
            after in "[rwxXt]{0,3}",
        ) {
            let mode = format!("{who}{op}{before}s{after}");
            prop_assert!(mode_sets_an_id_bit(&mode), "{}", mode);
        }

        #[test]
        fn a_mode_without_s_or_high_octal_bits_sets_none(
            who in "[ugoa]{0,3}",
            op in prop_oneof![Just('+'), Just('='), Just('-')],
            perms in "[rwxXt]{0,4}",
        ) {
            let mode = format!("{who}{op}{perms}");
            prop_assert!(!mode_sets_an_id_bit(&mode), "{}", mode);
        }
    }
}
