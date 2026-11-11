use crate::parse::Token;
use crate::registry;
use crate::verdict::Verdict;

pub(crate) fn check_open(tokens: &[Token]) -> Verdict {
    if launches_an_operand(tokens) {
        return Verdict::Denied;
    }
    registry::try_fallback_grammar("open", tokens).unwrap_or(Verdict::Denied)
}

fn launches_an_operand(tokens: &[Token]) -> bool {
    let mut as_document = false;
    let mut operands = Vec::new();
    let mut after_separator = false;
    let mut url_value_next = false;
    for token in &tokens[1..] {
        let word = token.as_str();
        if url_value_next {
            url_value_next = false;
            if !is_web_address(word) {
                return true;
            }
        } else if after_separator || !operands.is_empty() || !word.starts_with('-') || word == "-" {
            operands.push(word);
        } else if word == "--" {
            after_separator = true;
        } else if !word.starts_with("--") {
            let letters = &word[1..];
            if letters.contains(['R', 'e', 't']) {
                as_document = true;
            }
            if letters.contains('u') {
                if !letters.ends_with('u') {
                    return true;
                }
                url_value_next = true;
            }
        }
    }
    operands.iter().any(|operand| has_a_scheme(operand) && !is_web_address(operand))
        || (!as_document && operands.iter().any(|operand| !is_web_address(operand)))
}

fn has_a_scheme(word: &str) -> bool {
    word.split_once(':').is_some_and(|(scheme, _)| {
        scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
    })
}

fn is_web_address(word: &str) -> bool {
    let lower = word.to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://")
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        open_https: "open https://example.com",
        open_http_localhost: "open http://localhost:8000/",
        open_upper_scheme: "open HTTPS://example.com",
        open_reveal: "open -R target/release/app",
        open_reveal_app: "open -R Build/Foo.app",
        open_reveal_a_name_with_a_colon_after_a_digit: "open -R 1a:b",
        open_reveal_a_name_with_a_colon_after_a_path: "open -R dir/a:b",
        open_reveal_a_name_with_a_colon_after_an_underscore: "open -R a_b:c",
        open_textedit: "open -e notes.command",
        open_text_editor: "open -t script.sh",
        open_clustered_reveal: "open -gR x.command",
        open_stdin_in_editor: "open -f",
        open_wait_url: "open -W https://example.com",
        open_url_flag_web: "open -u https://example.com",
        open_new_url: "open -n https://example.com",
    }

    denied! {
        open_reveal_after_the_operand: "open x.command -R",
        open_editor_after_the_operand: "open x.command -e",
        open_text_after_the_operand: "open Foo.app -t",
        open_posix_order_editor_after_the_operand: "POSIXLY_CORRECT=1 open x.command -e",
        open_command_file: "open x.command",
        open_command_absolute: "/usr/bin/open x.command",
        open_command_quoted: "open 'x.command'",
        open_command_env_prefix: "FOO=1 open x.command",
        open_script_file: "open x.sh",
        open_app: "open Foo.app",
        open_document: "open README.md",
        open_dot: "open .",
        open_in_terminal: "open -a Terminal x.sh",
        open_app_by_name_with_url: "open -a Terminal https://example.com",
        open_bundle_id: "open -b com.apple.Terminal x.sh",
        open_env: "open --env DYLD_INSERT_LIBRARIES=x.dylib Foo.app",
        open_env_url: "open --env DYLD_INSERT_LIBRARIES=x.dylib https://example.com",
        open_stdout: "open --stdout out.txt https://example.com",
        open_url_flag_file: "open -u file:///tmp/x.command",
        open_url_flag_file_with_editor: "open -e -u file:///tmp/x.command",
        open_url_flag_clustered: "open -uW https://example.com x.command",
        open_after_separator: "open -- -x.command",
        open_url_then_file: "open https://example.com x.command",
        open_file_url: "open file:///tmp/x.command",
        open_new_instance_app: "open -n Foo.app",
        open_stdin_redirect: "open --stdin in.txt Foo.app",
        open_stdin_file_operand: "open -f x.command",
        open_args: "open https://example.com --args -x",
        open_long_reveal: "open --reveal x.command",
        open_subshell: "(open x.command)",
        open_after_chain: "ls && open x.command",
        open_reveal_ssh: "open -R ssh://host",
        open_editor_man_page: "open -e x-man-page://ls",
        open_text_vnc: "open -t vnc://host",
        open_reveal_file_url: "open -R file:///tmp/x.command",
        open_reveal_upper_file_url: "open -R FILE:///tmp/x.command",
        open_reveal_scheme_without_slashes: "open -R https:x.command",
        open_reveal_after_separator_scheme: "open -R -- ssh://host",
        open_editor_opaque_scheme: "open -e mailto:x@example.com",
    }

    use super::{Token, Verdict, check_open};
    use proptest::prelude::*;

    fn toks(words: &[String]) -> Vec<Token> {
        words.iter().map(|w| Token::from_test(w)).collect()
    }

    proptest! {
        #[test]
        fn an_operand_that_is_not_a_web_address_is_denied_without_a_document_flag(
            flags in proptest::collection::vec(prop_oneof![Just("-W"), Just("-n"), Just("-g"), Just("-j"), Just("-F"), Just("-f"), Just("-h")], 0..3),
            name in "[a-zA-Z0-9_./~-]{1,12}",
            ext in prop_oneof![Just(""), Just(".command"), Just(".app"), Just(".sh"), Just(".md"), Just(".terminal")],
        ) {
            let operand = format!("{name}{ext}");
            prop_assume!(!operand.starts_with('-'));
            let mut words = vec!["open".to_string()];
            words.extend(flags.iter().map(|f| (*f).to_string()));
            words.push(operand);
            prop_assert_eq!(check_open(&toks(&words)), Verdict::Denied);
        }

        #[test]
        fn a_document_flag_lets_the_same_operand_through(
            flag in prop_oneof![Just("-R"), Just("-e"), Just("-t")],
            name in "[a-zA-Z0-9_]{1,12}",
            ext in prop_oneof![Just(""), Just(".command"), Just(".app"), Just(".sh"), Just(".md")],
        ) {
            let words = vec!["open".to_string(), flag.to_string(), format!("{name}{ext}")];
            prop_assert!(check_open(&toks(&words)).is_allowed());
        }

        #[test]
        fn a_url_flag_value_must_be_a_web_address_even_with_a_document_flag(
            document in prop_oneof![Just(None), Just(Some("-R")), Just(Some("-e")), Just(Some("-t"))],
            scheme in prop_oneof![Just("file:///"), Just("ssh://"), Just("x-terminal://"), Just(""), Just("vnc://")],
            rest in "[a-z0-9./]{1,12}",
        ) {
            let mut words = vec!["open".to_string()];
            words.extend(document.map(str::to_string));
            words.push("-u".to_string());
            words.push(format!("{scheme}{rest}"));
            prop_assert_eq!(check_open(&toks(&words)), Verdict::Denied);
        }

        /// An address with a scheme is handed to the scheme's handler, whatever a document flag
        /// says, so only a web address passes as an operand.
        #[test]
        fn an_operand_with_a_scheme_other_than_the_web_is_denied_with_any_flag(
            document in prop_oneof![Just(None), Just(Some("-R")), Just(Some("-e")), Just(Some("-t")), Just(Some("-W"))],
            scheme in "[a-zA-Z][a-zA-Z0-9+.-]{0,8}",
            slashes in prop_oneof![Just(""), Just("//"), Just("///")],
            rest in "[a-z0-9./]{1,12}",
        ) {
            let lower = scheme.to_ascii_lowercase();
            prop_assume!(!((lower == "http" || lower == "https") && slashes.len() == 2));
            let mut words = vec!["open".to_string()];
            words.extend(document.map(str::to_string));
            words.push(format!("{scheme}:{slashes}{rest}"));
            prop_assert_eq!(check_open(&toks(&words)), Verdict::Denied);
        }
    }
}
