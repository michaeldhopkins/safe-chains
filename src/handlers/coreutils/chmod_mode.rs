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

pub(crate) fn mkdir_mode_sets_an_id_bit(tokens: &[Token]) -> bool {
    let mut words = tokens[1..].iter().map(Token::as_str);
    while let Some(word) = words.next() {
        if word == "--" {
            return false;
        }
        let value = if word == "--mode" {
            words.next()
        } else if let Some(value) = word.strip_prefix("--mode=") {
            Some(value)
        } else if !word.starts_with("--") && word.len() > 1 && word.starts_with('-') && word.contains('m') {
            let rest = &word[word.find('m').map_or(word.len(), |at| at + 1)..];
            if rest.is_empty() { words.next() } else { Some(rest) }
        } else {
            continue;
        };
        if value.is_none_or(mode_sets_an_id_bit) {
            return true;
        }
    }
    false
}

pub(crate) fn rsync_chmod_sets_an_id_bit(tokens: &[Token]) -> bool {
    let mut words = tokens[1..].iter().map(Token::as_str);
    while let Some(word) = words.next() {
        let value = if word == "--chmod" {
            words.next()
        } else if let Some(value) = word.strip_prefix("--chmod=") {
            Some(value)
        } else {
            continue;
        };
        let Some(value) = value else { return true };
        if value
            .split(',')
            .any(|clause| mode_sets_an_id_bit(clause.strip_prefix(['D', 'F']).unwrap_or(clause)))
        {
            return true;
        }
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
    edits_an_acl(mode) || mode.split(',').any(clause_sets_an_id_bit)
}

fn edits_an_acl(mode: &str) -> bool {
    mode.as_bytes().windows(2).any(|pair| matches!(pair[0], b'+' | b'-' | b'=') && pair[1] == b'a')
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
        mkdir_mode_plain: "mkdir -m 755 d",
        mkdir_mode_cluster_plain: "mkdir -pm 0755 d",
        mkdir_mode_symbolic: "mkdir --mode=u+rwx d",
        mkdir_parents: "mkdir -p a/b",
        rsync_chmod_plain: "rsync --chmod=Du+rwx,Fgo-w src/ dst/",
        rsync_chmod_octal: "rsync --chmod=F644 a b",
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
        chmod_acl_add: "chmod +a 'everyone allow write' x",
        chmod_acl_replace: "chmod =a# 0 'everyone deny read' x",
        chmod_acl_add_at: "chmod +a# 0 'everyone allow read' x",
        chmod_acl_inherited: "chmod +ai 'everyone allow read' x",
        chmod_acl_remove: "chmod -a 'everyone deny read' x",
        chmod_acl_from_stdin: "chmod -E x",
        mkdir_mode_setuid: "mkdir -m 4755 d",
        mkdir_mode_attached: "mkdir -m4755 d",
        mkdir_mode_long_equals: "mkdir --mode=u+s d",
        mkdir_mode_long_spaced: "mkdir --mode g+s d",
        mkdir_mode_cluster: "mkdir -pm 2775 d",
        mkdir_mode_cluster_attached: "mkdir -pm2775 d",
        mkdir_mode_before_parents: "mkdir -m 2775 -p d",
        mkdir_gnu_alias: "gmkdir -m u+s d",
        rsync_chmod_setuid: "rsync --chmod=u+s a b",
        rsync_chmod_spaced: "rsync --chmod Du+s a b",
        rsync_chmod_octal_file: "rsync --chmod=F4755 a b",
        rsync_chmod_list: "rsync -a --chmod=D2775,F644 src/ dst/",
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

        /// Every spelling of a mode given to mkdir or rsync is judged the same way chmod judges it.
        #[test]
        fn a_mode_given_to_mkdir_or_rsync_is_judged_as_chmod_judges_it(
            id_bit in prop::bool::ANY,
            perms in "[rwx]{1,3}",
            spelling in 0usize..7,
        ) {
            let mode = if id_bit { format!("u+{perms}s") } else { format!("u+{perms}") };
            let line = match spelling {
                0 => format!("mkdir -m {mode} d"),
                1 => format!("mkdir -m{mode} d"),
                2 => format!("mkdir --mode={mode} d"),
                3 => format!("mkdir --mode {mode} d"),
                4 => format!("mkdir -pm {mode} d"),
                5 => format!("rsync --chmod=D{mode},F644 src/ dst/"),
                _ => format!("rsync --chmod {mode} a b"),
            };
            prop_assert_eq!(is_safe_command(&line), !id_bit, "{}", line);
        }
    }
}
