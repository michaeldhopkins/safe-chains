use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static CONVERT_FORMATS: WordSet = WordSet::new(&["binary1", "json", "swift", "xml1"]);
static EXTRACT_FORMATS: WordSet = WordSet::new(&["binary1", "json", "raw", "xml1"]);

pub fn check_plutil_convert(tokens: &[Token]) -> Verdict {
    if tokens.len() == 2 && matches!(tokens[1].as_str(), "--help" | "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let Some(fmt) = tokens.get(1) else {
        return Verdict::Denied;
    };
    if !CONVERT_FORMATS.contains(fmt) {
        return Verdict::Denied;
    }
    walk_modifier_args(tokens, 2)
}

pub fn check_plutil_extract(tokens: &[Token]) -> Verdict {
    if tokens.len() == 2 && matches!(tokens[1].as_str(), "--help" | "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let Some(_keypath) = tokens.get(1) else {
        return Verdict::Denied;
    };
    let Some(fmt) = tokens.get(2) else {
        return Verdict::Denied;
    };
    if !EXTRACT_FORMATS.contains(fmt) {
        return Verdict::Denied;
    }
    walk_modifier_args(tokens, 3)
}

fn walk_modifier_args(tokens: &[Token], start: usize) -> Verdict {
    let mut i = start;
    let mut output_mode = OutputMode::InPlace;
    while i < tokens.len() {
        match tokens[i].as_str() {
            "-r" | "-s" => i += 1,
            "-o" => {
                let Some(value) = tokens.get(i + 1) else {
                    return Verdict::Denied;
                };
                if value.as_str().starts_with("--") {
                    return Verdict::Denied;
                }
                output_mode = if value.as_str() == "-" { OutputMode::Stdout } else { OutputMode::File };
                i += 2;
            }
            "--" => break,
            arg if arg.starts_with('-') => return Verdict::Denied,
            _ => i += 1,
        }
    }
    match output_mode {
        OutputMode::Stdout => Verdict::Allowed(SafetyLevel::Inert),
        OutputMode::File | OutputMode::InPlace => Verdict::Allowed(SafetyLevel::SafeWrite),
    }
}

enum OutputMode {
    Stdout,
    File,
    InPlace,
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        convert_stdout_xml: "plutil -convert xml1 -o - /tmp/foo.plist",
        convert_stdout_binary: "plutil -convert binary1 -o - /tmp/foo.plist",
        convert_stdout_json: "plutil -convert json -o - /tmp/foo.plist",
        convert_stdout_swift: "plutil -convert swift -o - /tmp/foo.plist",
        convert_stdout_stdin: "plutil -convert xml1 -o -",
        convert_stdout_readable: "plutil -convert json -r -o - /tmp/foo.plist",
        convert_stdout_silent: "plutil -convert xml1 -s -o - /tmp/foo.plist",
        convert_stdout_multi_input: "plutil -convert xml1 -o - -- /tmp/a.plist /tmp/b.plist",
        convert_stdout_piped: "plutil -convert xml1 -o - /tmp/foo.plist | grep boundary",
        convert_to_file: "plutil -convert json Info.plist -o out.json",
        convert_to_file_explicit: "plutil -convert xml1 -o /tmp/out.plist /tmp/in.plist",
        convert_in_place: "plutil -convert xml1 /tmp/foo.plist",
        convert_help: "plutil -convert --help",
        convert_help_short: "plutil -convert -h",
        extract_raw: "plutil -extract CFBundleVersion raw Info.plist",
        extract_json: "plutil -extract CFBundleShortVersionString json Info.plist",
        extract_to_file: "plutil -extract CFBundleURLTypes xml1 -o urls.plist Info.plist",
        extract_to_stdout: "plutil -extract CFBundleURLTypes xml1 -o - Info.plist",
        extract_help: "plutil -extract --help",
    }

    denied! {
        convert_bad_format: "plutil -convert invalid -o - /tmp/foo.plist",
        convert_objc_removed: "plutil -convert objc -o - /tmp/foo.plist",
        convert_no_format: "plutil -convert",
        convert_dash_dash_as_o_value: "plutil -convert xml1 -o -- /tmp/in",
        convert_extension_flag: "plutil -convert xml1 -e plist -o - /tmp/in",
        convert_unknown_flag: "plutil -convert xml1 -o - --unknown /tmp/in",
        extract_no_keypath: "plutil -extract",
        extract_bad_format: "plutil -extract Key bogus Info.plist",
        extract_unknown_flag: "plutil -extract Key json --evil Info.plist",
    }

    use proptest::prelude::*;

    use super::check_plutil_convert;
    use crate::parse::Token;
    use crate::verdict::{SafetyLevel, Verdict};

    /// A token that is benign in the modifier region: a non-empty run that does NOT start with
    /// `-`, so the walker treats it as an input path (skipped) rather than a flag. Restricting the
    /// modifier region to these keeps unknown-flag denials out of the properties below, so each one
    /// can pin the dimension it actually cares about.
    fn arb_benign_path_token() -> impl Strategy<Value = Token> {
        "[a-zA-Z0-9._/]{1,16}".prop_map(|s| Token::from_raw(s.to_string()))
    }

    /// A non-empty value that does not start with `-`, suitable as the argument to `-o`
    /// (so `walk_modifier_args` classifies it as `OutputMode::File`, not `Stdout`).
    fn arb_o_value_other_than_dash() -> impl Strategy<Value = Token> {
        prop_oneof![
            Just(Token::from_raw("out".to_string())),
            Just(Token::from_raw("a.json".to_string())),
            Just(Token::from_raw("foo.plist".to_string())),
            "[a-zA-Z0-9._/]{1,8}".prop_map(|s| Token::from_raw(s.to_string())),
        ]
    }

    /// A single `-r` or `-s` flag as a `Token`.
    fn arb_rs_token() -> impl Strategy<Value = Token> {
        proptest::sample::select(vec!["-r", "-s"]).prop_map(|s| Token::from_raw(s.to_string()))
    }

    /// A 2-token convert slice shaped `[-convert, <fmt>]`. The function is only ever asked about
    /// the format at `tokens[1]`, so the slice leading is just the sub-name and the format.
    fn arb_convert_two_token_slice() -> impl Strategy<Value = Vec<Token>> {
        proptest::sample::select(vec!["binary1", "json", "swift", "xml1"])
            .prop_map(|fmt| vec![Token::from_raw("-convert".to_string()), Token::from_raw(fmt.to_string())])
    }

    /// Tokens suitable for the modifier region after `-convert <fmt>`, restricted so the only `-o`
    /// in the invocation is whatever the property places explicitly. Lets the properties below pin
    /// the single `-o` semantics: any `-o` from the appended tail would overwrite the explicit one,
    /// making `Inert` ↔ `SafeWrite` comparisons meaningless. The walker just needs to handle three
    /// things to keep these properties clear of unknown-flag noise: `-r` / `-s` toggles, benign
    /// paths, or nothing.
    fn arb_o_free_modifier_tokens() -> impl Strategy<Value = Vec<Token>> {
        prop_oneof![
            // No modifiers at all: bare `convert <fmt>` invocation.
            Just(Vec::new()),
            // `-r` and `-s` are no-arg toggles; emit any sequence of them.
            proptest::collection::vec(arb_rs_token(), 0..4),
            // A single benign trailing token (treated as a path).
            arb_benign_path_token().prop_map(|t| vec![t]),
            // Several benign trailing tokens.
            proptest::collection::vec(arb_benign_path_token(), 1..3),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        /// Invariant — a 2-token convert slice whose second slot is `--help` or `-h` short-circuits
        /// to `Allowed(Inert)` before any format allowlist or modifier walk can deny. The length-2
        /// guard exists because the dispatcher normally routes `plutil --help` at the bare-flag
        /// layer; the in-handler check is the belt to that suspenders.
        #[test]
        fn convert_help_or_h_in_two_token_slice_is_inert(
            help in proptest::sample::select(vec!["--help", "-h"]),
        ) {
            let tokens = vec![Token::from_raw("-convert".to_string()), Token::from_raw(help.to_string())];
            prop_assert_eq!(
                check_plutil_convert(&tokens),
                Verdict::Allowed(SafetyLevel::Inert),
                "convert help: tokens={:?}",
                tokens.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
            );
        }

        /// Invariant — anything that is NOT a help token AND NOT in the convert format allowlist
        /// is denied. Closes the exact path the `objc` removal test pins: removing a format from
        /// the allowlist (an in-tree cleanup) and forgetting to refresh every constant in the
        /// handler will silently let that format through if this property is absent.
        #[test]
        fn convert_format_outside_allowlist_is_denied_for_any_modifier_tail(
            base in arb_convert_two_token_slice(),
            fmt_off in "[a-zA-Z0-9_]{1,8}",
            // Modifier tail: any benign mix so the property covers the "format bad, modifiers OK"
            // AND the "format bad, no modifiers" cases.
            tail in arb_o_free_modifier_tokens(),
        ) {
            // Force `fmt_off` to be outside the allowlist by the size of its class: 16 candidates
            // (4 letter × 4 length buckets) versus 4 allowlisted formats means p(collision) < 25%.
            let tokens_already = base[1].as_str().to_string();
            let mut off = Token::from_raw(fmt_off.clone());
            while off.as_str() == tokens_already || matches!(off.as_str(), "binary1" | "json" | "swift" | "xml1") {
                off = Token::from_raw(format!("{}_x", fmt_off));
            }

            // Defensive: shrink gave us a rewrap starting with the same letter but not matching.
            // Rebuild the slice with `[sub-name, bad-format, modifiers...]`.
            let mut tokens = vec![Token::from_raw("-convert".to_string()), off.clone()];
            tokens.extend(tail);
            prop_assert_eq!(
                check_plutil_convert(&tokens),
                Verdict::Denied,
                "convert bad format: tokens={:?}",
                tokens.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
            );
        }

        /// Differential / oracle — the load-bearing `-o` semantics: value `-` means write to stdout
        /// (Inert), any other benign value means a file target (SafeWrite). The other modifiers
        /// (`-r`, `-s`, bare trailing tokens) do not influence the safety level. A regression that
        /// classifies `-o -` as SafeWrite (or any non-dash value as Inert) is exactly the
        /// write-mode misclass the file-output branch exists to prevent.
        #[test]
        fn convert_dash_o_dash_is_inert_and_dash_o_other_value_is_safe_write(
            base in arb_convert_two_token_slice(),
            modifiers in arb_o_free_modifier_tokens(),
            non_dash in arb_o_value_other_than_dash(),
        ) {
            // Build two convert invocations that differ only in the value passed to `-o`.
            let mut with_dash = base.clone();
            with_dash.push(Token::from_raw("-o".to_string()));
            with_dash.push(Token::from_raw("-".to_string()));
            with_dash.extend(modifiers.iter().cloned());

            let mut with_other = base;
            with_other.push(Token::from_raw("-o".to_string()));
            with_other.push(non_dash);
            with_other.extend(modifiers);

            // When the value is `-`, output is stdout, so the verdict is Inert.
            prop_assert_eq!(
                check_plutil_convert(&with_dash),
                Verdict::Allowed(SafetyLevel::Inert),
                "with_dash tokens={:?}",
                with_dash.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
            );

            // When the value is anything else (no leading `-`, never `--`), the modifier walk
            // finishes in `OutputMode::File` and yields SafeWrite.
            let non_dash_value = with_other[3].as_str();
            prop_assert!(
                !non_dash_value.starts_with('-'),
                "arb_o_value_other_than_dash leaked a `-`-value: {:?}", non_dash_value,
            );
            prop_assert_eq!(
                check_plutil_convert(&with_other),
                Verdict::Allowed(SafetyLevel::SafeWrite),
                "with_other tokens={:?}",
                with_other.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
            );
        }

        /// Metamorphic — `-r` and `-s` are documented no-arg toggles (readable / silent) with
        /// identical walker behaviour: each is consumed and the pointer advances by one. Their
        /// ORDER therefore cannot change the verdict: reversing them yields the same safety
        /// classification. A regression that special-cased one of them, or that conflated them
        /// with `-o`, would flip the verdict on at least one shuffle.
        #[test]
        fn convert_reordering_dash_r_and_dash_s_preserves_the_verdict(
            base in arb_convert_two_token_slice(),
            // 1..6 flags, each either `-r` or `-s`, followed by an optional benign trailer.
            picks in proptest::collection::vec(arb_rs_token(), 1..6),
            trailer in proptest::collection::vec(arb_benign_path_token(), 0..3),
        ) {
            // Original order.
            let mut original = base.clone();
            original.extend(picks.iter().cloned());
            original.extend(trailer.iter().cloned());

            // Reversed — every flag swapped end for end, trailer held constant.
            let mut reversed = base;
            let mut picks_rev = picks.clone();
            picks_rev.reverse();
            reversed.extend(picks_rev);
            reversed.extend(trailer);

            prop_assert_eq!(
                check_plutil_convert(&original),
                check_plutil_convert(&reversed),
                "reordering -r/-s must not change verdict",
            );
        }
    }
}
