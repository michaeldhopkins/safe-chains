use super::eval::{UNPINNABLE, UNRESOLVED_BRACE};
use super::{Cmd, parse};
use proptest::prelude::*;

const REQUIRED_BASH: &str = "/bin/bash";
const OTHER_BASHES: &[&str] = &["/opt/homebrew/bin/bash", "/usr/local/bin/bash", "/usr/bin/bash"];

fn bashes() -> Vec<&'static str> {
    assert!(std::path::Path::new(REQUIRED_BASH).is_file(), "{REQUIRED_BASH} is required to check brace expansion against bash");
    let mut found = vec![REQUIRED_BASH];
    for path in OTHER_BASHES {
        let same = std::fs::canonicalize(path)
            .ok()
            .zip(std::fs::canonicalize(REQUIRED_BASH).ok())
            .is_some_and(|(a, b)| a == b);
        if std::path::Path::new(path).is_file() && !same {
            found.push(path);
        }
    }
    found
}

fn bash_words(bash: &str, word: &str) -> Vec<String> {
    let script = format!("set -f; printf '%s\\n' {word}");
    let out = std::process::Command::new(bash)
        .args(["--noprofile", "--norc", "-c", &script])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .output()
        .unwrap_or_else(|err| panic!("{bash}: {err}"));
    assert!(out.status.success(), "{bash} rejected {word}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout)
        .expect("utf-8")
        .split_terminator('\n')
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

fn our_words(word: &str) -> Vec<String> {
    let script = parse(&format!("printf '%s\\n' {word}")).unwrap_or_else(|| panic!("parse {word}"));
    let Cmd::Simple(cmd) = &script.0[0].pipeline.commands[0] else {
        panic!("{word} is not a simple command");
    };
    cmd.words[2..].iter().flat_map(super::Word::expand).filter(|w| !w.is_empty()).collect()
}

fn fails_closed(words: &[String]) -> bool {
    words.iter().any(|w| w.contains(UNRESOLVED_BRACE) || w.contains(UNPINNABLE) || w.contains('$'))
}

/// A word our expander either declines (unresolved) or expands to exactly what every bash here
/// prints. An exact answer that one bash disagrees with is a word the classifier misjudges.
fn mismatch(word: &str) -> Option<String> {
    let ours = our_words(word);
    if fails_closed(&ours) {
        return None;
    }
    bashes().into_iter().find_map(|bash| {
        let theirs = bash_words(bash, word);
        (theirs != ours).then(|| format!("{word}: ours {ours:?}, {bash} {theirs:?}"))
    })
}

#[test]
fn the_reviewed_brace_spellings_are_exact_or_unresolved() {
    let words = [
        r#".g{"i",x}t"#, r#"{".git",notes}"#, "{'.git',notes}", r#"{.git,"notes"}"#, "{$'.git',notes}", r"{.git,no\,tes}",
        r#".g{"i",x}t/config"#, ".gi{t..t..1}", "log.{01..01}", "{01..03}", "{1..05}", "{1..10..2}", "{a..e..2}", "{1..2{a,b}}", r"\{a,b}",
        r"{a,b\}", r".gi\{t..t\}", "'.gi{t..t}'", r#"".gi"{t..t}"#, "'.gi'{t..t}", "log.{1..3}", "{a,{b,c}}", "{a,{1..3},b}", "{1..3,5}",
        "{a..Z}", "{-2..2}", "{+1..3}", "{h..a}", "{0..0..2}", "x{1..2}y{3..4}", "{ab..cd}", "{1..a}", "w{..a}{b,c}", "w{a..1}{b,c}",
        "{..a{b,c}}x", "{sub/../.git{,}}", "{a{b,c}..d}", "{..a{b..c}}", "{...}{1..2}+", r#"w{..a}"x"{b,c}"#, "w{..a}{1..2}", r"w\ {},x}",
        "w{a..1},x}", "w{ab..cd},x}", "w{..a},x}", "w{1..2/},x}", "w{../},x}", "w{...},x}", r"w{a..\}},x}", "w{..},x}", "w{a..},x}",
        "w{a.b},x}", "w{a..b..},x}", "w{a{1..x}b},x}", r"w{a..\},x}", r"w{.\.x},y}", r#"w{"..x"},y}"#, r"w{a\ {},x}",
        r#"./{"../../.ssh/config",x}"#,
    ];
    let bad: Vec<String> = words.iter().filter_map(|word| mismatch(word)).collect();
    assert!(bad.is_empty(), "brace words judged differently from bash:\n{}", bad.join("\n"));
}

fn arb_token() -> impl Strategy<Value = String> {
    let plain = "[ab.,{}0-2/A-]{1,3}";
    let quoted = "[ab.,{}]{0,2}";
    prop_oneof![
        4 => prop::string::string_regex(plain).expect("regex"),
        2 => prop_oneof![Just("{".to_string()), Just("}".to_string()), Just(",".to_string())],
        1 => prop_oneof![Just("..2".to_string()), Just("..".to_string()), Just("...".to_string())],
        1 => prop_oneof![Just(r"\ ".to_string()), Just(r"\.".to_string()), Just("},x}".to_string()), Just("w{".to_string()), Just("{..a}".to_string()), Just("{a..1}".to_string())],
        1 => Just("..2".to_string()),
        1 => Just("0".to_string()),
        1 => prop::string::string_regex(quoted).expect("regex").prop_map(|s| format!("\"{s}\"")),
        1 => prop::string::string_regex(quoted).expect("regex").prop_map(|s| format!("'{s}'")),
        1 => prop::string::string_regex("[ab.,{}]").expect("regex").prop_map(|s| format!("\\{s}")),
        1 => prop::string::string_regex("[ab.,]{0,2}").expect("regex").prop_map(|s| format!("$'{s}'")),
        1 => prop_oneof![Just("$x".to_string()), Just("${x}".to_string()), Just("${x,}".to_string())],
    ]
}

fn arb_brace_word() -> impl Strategy<Value = String> {
    prop::collection::vec(arb_token(), 1..7)
        .prop_map(|tokens| tokens.concat())
        .prop_filter("a brace to expand", |w| w.contains('{'))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    /// Brace expansion runs over the whole word, with quoted and escaped characters taken
    /// literally. Every word the expander answers exactly matches every bash on this machine,
    /// whatever its version; a word it cannot model is unresolved.
    #[test]
    fn a_brace_word_is_exact_or_unresolved(word in arb_brace_word()) {
        prop_assert_eq!(mismatch(&word), None);
    }
}

fn pad_num(n: i32, width: usize) -> String {
    let digits = n.unsigned_abs().to_string();
    let body = if width > digits.len() { format!("{digits:0>width$}") } else { digits };
    if n < 0 { format!("-{body}") } else { body }
}

fn small_span(start: i32, end: i32, step: i32) -> bool {
    let step = u64::from(step.unsigned_abs()).max(1);
    u64::from(start.abs_diff(end)) / step < 8
}

fn arb_numeric() -> impl Strategy<Value = String> {
    (1usize..=4, -12i32..=12, -12i32..=12, 0i32..=4, any::<bool>()).prop_filter_map(
        "span stays inside the expansion cap",
        |(width, start, end, step, reversed)| {
            let (start, end) = if reversed { (end, start) } else { (start, end) };
            small_span(start, end, step).then_some(())?;
            let left = pad_num(start, width);
            let right = pad_num(end, width);
            Some(if step == 1 { format!("{{{left}..{right}}}") } else { format!("{{{left}..{right}..{step}}}") })
        },
    )
}

fn arb_letter() -> impl Strategy<Value = String> {
    (prop::char::range('A', 'z'), prop::char::range('A', 'z'), 0i32..=3).prop_filter_map("letter ends", |(start, end, step)| {
        (start.is_ascii_alphabetic() && end.is_ascii_alphabetic()).then_some(())?;
        Some(if step <= 1 { format!("{{{start}..{end}}}") } else { format!("{{{start}..{end}..{step}}}") })
    })
}

fn arb_invalid() -> impl Strategy<Value = String> {
    prop_oneof![
        (any::<i8>(), prop::char::range('a', 'z')).prop_map(|(n, c)| format!("{{{n}..{c}}}")),
        (prop::char::range('a', 'z'), any::<i8>()).prop_map(|(c, n)| format!("{{{c}..{n}}}")),
        (prop::char::range('a', 'z'), prop::char::range('a', 'z'), prop::char::range('a', 'z'))
            .prop_map(|(a, b, c)| format!("{{{a}{b}..{c}}}")),
        (prop::char::range('a', 'z'), prop::char::range('a', 'z'), prop::char::range('a', 'z'))
            .prop_map(|(a, b, c)| format!("{{{a}..{b}{c}}}")),
        (any::<i8>(), any::<i8>()).prop_map(|(a, b)| format!("{{{a}..{b}..x}}")),
    ]
}

fn arb_sequence_word() -> impl Strategy<Value = String> {
    let affix = || prop::string::string_regex("[A-Za-z0-9._]{0,3}").expect("regex");
    (affix(), prop_oneof![arb_numeric(), arb_letter(), arb_invalid()], affix())
        .prop_map(|(prefix, body, suffix)| format!("{prefix}{body}{suffix}"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]
    /// A sequence (padded, stepped, lettered, reversed or invalid) is exact or unresolved.
    #[test]
    fn a_sequence_brace_is_exact_or_unresolved(word in arb_sequence_word()) {
        prop_assert_eq!(mismatch(&word), None);
    }
}
