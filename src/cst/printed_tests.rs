use super::printed_item;
use crate::cst::check::UNKNOWN_ITEM;
use crate::is_safe_command;
use proptest::prelude::*;

#[test]
fn one_plain_word_is_the_item_itself() {
    assert_eq!(printed_item("echo", &["BEGIN{system(1)}"]), "BEGIN{system(1)}");
    assert_eq!(printed_item("echo", &["-n", "a.txt"]), "a.txt");
    assert_eq!(printed_item("echo", &["-neE", "a.txt"]), "a.txt");
    assert_eq!(printed_item("echo", &["-x"]), "-x");
    assert_eq!(printed_item("printf", &["a.txt"]), "a.txt");
    assert_eq!(printed_item("echo", &[]), "sc_item");
}

#[test]
fn text_xargs_or_printf_would_rewrite_is_unknown() {
    for args in [&["a.txt", "b.txt"][..], &["a b"], &["a'b"], &["a\"b"], &["a\\nb"], &[""], &["a.txt", "-n"]] {
        assert_eq!(printed_item("echo", args), UNKNOWN_ITEM, "{args:?}");
    }
    for args in [&["%s", "x"][..], &["100%"], &["-v", "x"], &["a\\n"]] {
        assert_eq!(printed_item("printf", args), UNKNOWN_ITEM, "{args:?}");
    }
}

#[test]
fn printed_program_text_reaching_xargs_is_classified_as_that_text() {
    for line in [
        "echo 'include \"m\"; f' | xargs -0 jq -n",
        "echo 'include \"m\"; f' | xargs -0 gojq -n",
        "echo 'include \"m\"; f' | xargs -0 jaq -n",
        "echo '\"m\" | modulemeta' | xargs -0 jq -n",
        "printf 'include \"m\"; f' | xargs -0 jq -n",
        "echo 'include \"m\"; f' | xargs -d '\\n' jq -n",
        "echo 'include \"m\"; f' | xargs -I{} jq -n {}",
        "echo 'load(\"/etc/passwd\")' | xargs -0 yq",
        "echo 'BEGIN{system(\"touch /tmp/x\")}' | xargs -0 awk",
        "echo 'BEGIN{system(1)}' | xargs awk",
        "echo '1e touch /tmp/x' | xargs -0 sed -n",
        "echo 1e | xargs -I{} sed {} ./a.txt",
        "echo x.e | xargs -d. -I{} sed {} ./a.txt",
        "echo 'include \"m\"; f' | while read -r p; do jq -n \"$p\"; done",
        "echo 'BEGIN{system(1)}' | while read -r p; do awk \"$p\"; done",
        "echo /etc/shadow | xargs cat",
    ] {
        assert!(!is_safe_command(line), "{line} must not approve");
    }
}

#[test]
fn printed_paths_still_reach_xargs() {
    for line in ["echo a.txt | xargs cat", "echo ./src | xargs ls", "echo hi | xargs -I{} echo {}", "echo a.txt | xargs jq .name"] {
        assert!(is_safe_command(line), "{line} must still approve");
    }
}

fn plain_word() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::sample::select(vec!["BEGIN{system(1)}", "1e", "e", "include", "modulemeta", ".name", "./a.txt", "/etc/passwd", "w./x"])
            .prop_map(str::to_string),
        "[a-zA-Z0-9{}().;/_]{1,12}",
    ]
}

proptest! {
    /// A word printed into xargs is approved only where the same word, written in place, would be:
    /// xargs never makes a word safer than it is.
    #[test]
    fn a_printed_word_is_never_safer_through_xargs(
        word in plain_word(),
        consumer in prop::sample::select(vec!["awk", "sed -n", "jq -n", "gojq -n", "yq", "cat", "ls", "grep -c x"]),
        via in prop::sample::select(vec!["echo", "echo -n", "printf"]),
        flags in prop::sample::select(vec!["", "-0 ", "-I{} ", "-d '\\n' "]),
    ) {
        let piped = format!("{via} {} | xargs {flags}{consumer}{}", shell_words::quote(&word), if flags.starts_with("-I") { " {}" } else { "" });
        let direct = format!("{consumer} {}", shell_words::quote(&word));
        if is_safe_command(&piped) {
            prop_assert!(is_safe_command(&direct), "{piped} approves but {direct} does not");
        }
    }
}

#[test]
fn xargs_replaces_only_what_it_replaces() {
    for line in [
        "echo cat | xargs -Irm rm ~/pwn",
        "echo ls | xargs -Itouch touch ~/pwn",
        "echo ls | xargs -Ish sh -c \"rm -rf /x\"",
        "echo f | xargs -I/etc rm /etc /etc /etc /etc /etc /etc/passwd",
        "echo foo | xargs -I.ssh cat .ssh .ssh .ssh .ssh .ssh ~/.ssh/id_rsa",
    ] {
        assert!(!is_safe_command(line), "{line} must not approve");
    }
    for line in ["echo a.txt | xargs -I{} cat {} {} {} {} {}", "echo a.txt | xargs -Icat cat cat"] {
        assert!(is_safe_command(line), "{line} must still approve");
    }
}

proptest! {
    /// xargs substitutes its replace string in the arguments, never in the utility, so naming the
    /// replace string after the utility never lets a printed word stand in for it.
    #[test]
    fn the_utility_is_never_replaced(
        utility in prop::sample::select(vec!["rm", "touch", "sh", "tee", "cp", "sed"]),
        item in prop::sample::select(vec!["cat", "ls", "echo"]),
        rest in prop::sample::select(vec!["~/pwn", "-c \"rm -rf /x\"", "./a ~/.zshrc", "-i s/a/b/ ~/.zshrc"]),
    ) {
        let piped = format!("echo {item} | xargs -I{utility} {utility} {rest}");
        let direct = format!("{utility} {}", rest.replace(utility, item));
        if is_safe_command(&piped) {
            prop_assert!(is_safe_command(&direct), "{piped} approves but {direct} does not");
        }
    }
}
