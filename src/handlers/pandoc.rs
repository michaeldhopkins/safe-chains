//! `pandoc` converts documents to stdout, with three spellings that run code or a program:
//! `pandoc lua` is a standalone Lua interpreter, `pandoc server` listens on a port, and the format
//! flags (`-f`/`-r` reader, `-t`/`-w` writer, `-D` template) load a Lua script when the value is a
//! path. `-t pdf` runs TeX. The first word and the format values are checked here; every other flag
//! is checked against the lists in `commands/tools/pandoc.toml`.
use crate::parse::Token;
use crate::policy::{FlagSet, takes_next_word};
use crate::registry;
use crate::verdict::Verdict;

const READER_LONG: &[&str] = &["--from", "--read"];
const WRITER_LONG: &[&str] = &["--to", "--write", "--print-default-template"];
const READER_SHORT: &[u8] = b"fr";
const WRITER_SHORT: &[u8] = b"twD";
const INPUT_FORMATS: &str = include_str!("../../tests/fixtures/pandoc/3.12.1/input-formats.txt");
const OUTPUT_FORMATS: &str = include_str!("../../tests/fixtures/pandoc/3.12.1/output-formats.txt");

pub fn check_pandoc(tokens: &[Token]) -> Verdict {
    if matches!(tokens.get(1).map(Token::as_str), Some("lua" | "server")) {
        return Verdict::Denied;
    }
    if tokens.iter().any(|t| is_runtime_option(t.as_str())) {
        return Verdict::Denied;
    }
    if !format_values_are_plain(tokens) {
        return Verdict::Denied;
    }
    registry::try_fallback_grammar("pandoc", tokens).unwrap_or(Verdict::Denied)
}

pub(super) fn is_runtime_option(token: &str) -> bool {
    token.starts_with("+RTS") || matches!(token, "-RTS" | "--RTS")
}

fn parse_format_list(raw: &str) -> Vec<&str> {
    raw.lines().map(str::trim).filter(|l| !l.is_empty()).collect()
}

fn is_reader_format(value: &str) -> bool {
    is_named_format(value, INPUT_FORMATS)
}

fn is_writer_format(value: &str) -> bool {
    is_named_format(value, OUTPUT_FORMATS)
}

fn is_named_format(value: &str, formats: &str) -> bool {
    is_plain_format(value) && value.split(['+', '-']).next().is_some_and(|base| parse_format_list(formats).contains(&base))
}

/// A format name with optional `+ext` / `-ext` suffixes: `[a-z0-9_]+([+-][a-z0-9_]+)*`. Nothing
/// with a `/` or `.` can name a script, and `pdf` runs a TeX engine.
fn is_plain_format(value: &str) -> bool {
    let mut previous_separator = true;
    for c in value.chars() {
        match c {
            'a'..='z' | '0'..='9' | '_' => previous_separator = false,
            '+' | '-' if !previous_separator => previous_separator = true,
            _ => return false,
        }
    }
    !previous_separator && value.split(['+', '-']).next() != Some("pdf")
}

fn is_any_format(value: &str) -> bool {
    is_reader_format(value) || is_writer_format(value)
}

fn long_format_check(name: &str) -> Option<fn(&str) -> bool> {
    if name == "--list-extensions" {
        Some(is_any_format)
    } else if READER_LONG.contains(&name) {
        Some(is_reader_format)
    } else if WRITER_LONG.contains(&name) {
        Some(is_writer_format)
    } else {
        None
    }
}

fn short_format_check(flag: u8) -> Option<fn(&str) -> bool> {
    if READER_SHORT.contains(&flag) {
        Some(is_reader_format)
    } else if WRITER_SHORT.contains(&flag) {
        Some(is_writer_format)
    } else {
        None
    }
}

fn format_values_are_plain(tokens: &[Token]) -> bool {
    let Some((standalone, valued)) = registry::fallback_flag_sets("pandoc") else {
        return false;
    };
    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        i += 1;
        if t == "--" {
            break;
        }
        if let Some(long) = t.strip_prefix("--") {
            let (name, inline) = match long.split_once('=') {
                Some((name, value)) => (format!("--{name}"), Some(value)),
                None => (t.to_string(), None),
            };
            let format_check = long_format_check(&name);
            if inline.is_none() && takes_next_word(standalone, valued, &name) {
                let value = tokens.get(i).map(Token::as_str);
                i += 1;
                if let Some(check) = format_check
                    && !value.is_some_and(check)
                {
                    return false;
                }
            } else if let Some(value) = inline
                && let Some(check) = format_check
                && !check(value)
            {
                return false;
            }
            continue;
        }
        if !t.starts_with('-') {
            continue;
        }
        let bytes = t.as_bytes();
        for (j, &b) in bytes.iter().enumerate().skip(1) {
            if standalone.contains_short(b) {
                continue;
            }
            let format_check = short_format_check(b);
            if valued.contains_short(b) {
                let value = if j + 1 < bytes.len() {
                    t.get(j + 1..)
                } else {
                    i += 1;
                    tokens.get(i - 1).map(Token::as_str)
                };
                if let Some(check) = format_check
                    && !value.is_some_and(check)
                {
                    return false;
                }
            }
            break;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use crate::policy::FlagSet;
    use proptest::prelude::*;

    use super::{is_plain_format, is_reader_format, is_writer_format};

    fn check(cmd: &str) -> bool {
        crate::is_safe_command(cmd)
    }
    use crate::verdict::{SafetyLevel, Verdict};

    safe! {
        plain_writer: "pandoc -t html README.md",
        writer_with_extensions: "pandoc -t gfm+smart-raw_html README.md",
        reader_glued: "pandoc -fmarkdown README.md",
        cluster_with_writer: "pandoc -st html README.md",
        template_by_format: "pandoc -D html",
        long_template_by_format: "pandoc --print-default-template=latex",
        a_file_named_like_a_flag_value: "pandoc -V title=lua README.md",
        writer_flag_as_value_of_long_variable: "pandoc --variable --to=lua README.md",
        writer_flag_as_value_of_short_variable: "pandoc -V --to=lua README.md",
        lua_flag_after_double_dash_is_operand: "pandoc -t html -- --to=lua",
        plain_writer_with_html_target: "pandoc README.md -t html",
        extension_on_known_reader: "pandoc -f commonmark_x+smart README.md",
        worktree_data_dir: "pandoc --data-dir=./data README.md",
        optional_value_flag_then_writer: "pandoc --toc -t html README.md",
        glued_optional_value_then_writer: "pandoc --toc=true -t html README.md",
        list_extensions_bare: "pandoc --list-extensions",
        list_extensions_of_reader: "pandoc --list-extensions=markdown",
        list_extensions_of_writer: "pandoc --list-extensions=gfm",
    }

    denied! {
        lua_interpreter: "pandoc lua x.lua",
        server: "pandoc server",
        lua_writer: "pandoc -t ./w.lua README.md",
        lua_bare_writer: "pandoc -t lua README.md",
        lua_suffixed_writer: "pandoc -t xlua README.md",
        lua_long_to_writer: "pandoc --to=my_lua README.md",
        lua_short_w_writer: "pandoc -w mylua README.md",
        lua_long_write_writer: "pandoc --write lua README.md",
        lua_clustered_writer: "pandoc -st lua README.md",
        lua_suffixed_writer_with_extension: "pandoc -t xlua+smart README.md",
        lua_reader_long: "pandoc --from=./r.lua README.md",
        lua_template: "pandoc -D d.lua",
        lua_bare_template: "pandoc -D lua",
        lua_writer_after_long_standalone: "pandoc --standalone --to=lua README.md",
        lua_writer_after_inline_long_valued: "pandoc --variable=x --to=lua README.md",
        lua_writer_after_glued_short_valued: "pandoc -Vx --to=lua README.md",
        foreign_data_dir_equals: "pandoc --data-dir=/tmp/evil README.md",
        foreign_data_dir_space: "pandoc --data-dir /tmp/evil README.md",
        home_data_dir: "pandoc --data-dir=~/.pandoc README.md",
        pdf_writer: "pandoc -t pdf README.md",
        missing_value: "pandoc README.md -t",
        equals_form_short: "pandoc -t=html README.md",
        trailing_separator: "pandoc -t html+ README.md",
        double_extension_separator: "pandoc -t html++smart README.md",
        runtime_options_stats_file: "pandoc README.md +RTS -tpwned",
        runtime_options_first: "pandoc +RTS -tpwned README.md",
        runtime_options_closed: "pandoc README.md +RTS -s -RTS",
        runtime_options_glued_file: "pandoc +RTS -sout -RTS README.md",
        runtime_options_after_separator: "pandoc -- README.md +RTS -tpwned",
        runtime_options_end_marker: "pandoc --RTS README.md",
        runtime_options_bare_end: "pandoc README.md -RTS",
        runtime_options_quoted: "pandoc README.md '+RTS' -tpwned",
        lua_writer_after_bare_optional_value_flag: "pandoc --toc -t ./w.lua in.md",
        lua_reader_after_bare_optional_value_flag: "pandoc --mathjax -f ./r.lua in.md",
        pdf_writer_after_bare_toc: "pandoc --toc -t pdf in.md",
        pdf_writer_after_bare_number_sections: "pandoc --number-sections -t pdf in.md",
        long_lua_writer_after_bare_optional_value_flag: "pandoc --toc --to ./w.lua in.md",
        glued_lua_writer_after_bare_optional_value_flag: "pandoc --toc --to=./w.lua in.md",
        list_extensions_of_a_lua_path: "pandoc --list-extensions=./w.lua",
        list_extensions_of_lua: "pandoc --list-extensions=lua",
        list_extensions_of_an_unknown_name: "pandoc --list-extensions=nosuchformat",
    }

    #[test]
    fn a_plain_format_is_read_only() {
        assert_eq!(crate::command_verdict("pandoc -t html README.md"), Verdict::Allowed(SafetyLevel::SafeRead));
    }

    #[test]
    fn every_format_flag_takes_a_value_in_the_flag_lists() {
        let (standalone, valued) = crate::registry::fallback_flag_sets("pandoc").expect("pandoc fallback");
        for flag in super::READER_LONG.iter().chain(super::WRITER_LONG) {
            assert!(valued.contains_flag(flag), "{flag} must be valued, or its value is never checked");
            assert!(!standalone.contains_flag(flag), "{flag} must not also be standalone");
        }
        for &short in super::READER_SHORT.iter().chain(super::WRITER_SHORT) {
            assert!(valued.contains_short(short), "-{} must be valued", short as char);
            assert!(!standalone.contains_short(short), "-{} must not also be standalone", short as char);
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum Arity {
        None,
        Optional,
        Required,
    }

    fn pandoc_arities() -> std::collections::BTreeMap<String, Arity> {
        let help = include_str!("../../tests/fixtures/pandoc/3.12.1/help.txt");
        let mut arities = std::collections::BTreeMap::new();
        for line in help.lines() {
            let words: Vec<&str> = line.split([' ', ',']).filter(|w| !w.is_empty()).collect();
            for (k, word) in words.iter().enumerate() {
                if let Some(long) = word.strip_prefix("--") {
                    let end = long.find(['=', '[']).unwrap_or(long.len());
                    let arity = match long.as_bytes().get(end) {
                        Some(b'=') => Arity::Required,
                        Some(b'[') => Arity::Optional,
                        _ => Arity::None,
                    };
                    arities.insert(format!("--{}", &long[..end]), arity);
                } else if word.len() >= 2 && word.starts_with('-') && word.as_bytes()[1].is_ascii_alphabetic() {
                    let arity = if word.as_bytes().get(2) == Some(&b'[') {
                        Arity::Optional
                    } else if word.len() == 2 && words.get(k + 1).is_some_and(|n| n.starts_with(|c: char| c.is_ascii_uppercase())) {
                        Arity::Required
                    } else {
                        Arity::None
                    };
                    arities.insert(word[..2].to_string(), arity);
                }
            }
        }
        arities
    }

    #[test]
    fn a_flag_that_takes_the_next_word_takes_a_required_value_in_pandoc() {
        let arities = pandoc_arities();
        assert!(arities.get("--toc") == Some(&Arity::Optional) && arities.get("-t") == Some(&Arity::Required), "{arities:?}");
        let (standalone, valued) = crate::registry::fallback_flag_sets("pandoc").expect("pandoc fallback");
        let mut wrong = Vec::new();
        for flag in valued.iter().chain(standalone.iter()) {
            let Some(&arity) = arities.get(flag.as_str()) else { continue };
            let takes_next = super::takes_next_word(standalone, valued, flag);
            if takes_next != (arity == Arity::Required) {
                wrong.push(format!("{flag}: pandoc {arity:?}, takes next word here: {takes_next}"));
            }
        }
        assert!(wrong.is_empty(), "flag lists disagree with pandoc --help:\n  {}", wrong.join("\n  "));
    }

    fn optional_value_flags() -> Vec<String> {
        let (standalone, valued) = crate::registry::fallback_flag_sets("pandoc").expect("pandoc fallback");
        standalone.iter().filter(|f| f.starts_with("--") && valued.contains_flag(f)).cloned().collect()
    }

    #[test]
    fn every_optional_value_flag_leaves_the_next_format_flag_checked() {
        let flags = optional_value_flags();
        assert!(flags.len() >= 20, "pandoc's optional-value flags were not found: {flags:?}");
        let mut approved = Vec::new();
        for flag in &flags {
            for format in [
                "-t ./w.lua", "-w ./w.lua", "--to ./w.lua", "--to=./w.lua", "--write ./w.lua", "-f ./r.lua", "-r ./r.lua",
                "--from ./r.lua", "--read=./r.lua", "-t pdf", "--to pdf", "-t lua", "-D ./t.lua",
            ] {
                let cmd = format!("pandoc {flag} {format} in.md");
                if check(&cmd) {
                    approved.push(cmd);
                }
            }
            let plain = format!("pandoc {flag} -t html in.md");
            assert!(check(&plain), "{plain} must stay approved");
        }
        assert!(approved.is_empty(), "approved:\n  {}", approved.join("\n  "));
    }

    proptest! {
        #[test]
        fn an_optional_value_flag_before_a_lua_or_pdf_format_is_refused(
            index in any::<prop::sample::Index>(),
            format_flag in prop::sample::select(vec!["-t", "-w", "-f", "-r", "-D", "--to", "--write", "--from", "--read"]),
            value in prop::sample::select(vec!["./w.lua", "w.lua", "lua", "xlua+smart", "pdf", "pdf+smart", "/tmp/w.lua"]),
            glued in any::<bool>(),
            before in prop::sample::select(vec!["", "-s", "--standalone", "-V x"]),
        ) {
            let flags = optional_value_flags();
            let flag = index.get(&flags);
            let format = if glued && format_flag.starts_with("--") { format!("{format_flag}={value}") } else { format!("{format_flag} {value}") };
            let cmd = format!("pandoc {before} {flag} {format} in.md");
            prop_assert!(!crate::is_safe_command(&cmd), "{}", cmd);
        }

        #[test]
        fn any_runtime_option_token_is_denied(
            before in "[a-zA-Z.]{0,6}", after in "[a-zA-Z0-9.-]{0,8}",
            marker in prop::sample::select(vec!["+RTS", "-RTS", "--RTS", "+RTSx"]),
        ) {
            let cmd = format!("pandoc {before} {marker} {after}");
            prop_assert!(!crate::is_safe_command(&cmd), "{}", cmd);
        }

        #[test]
        fn a_value_with_a_path_character_is_never_a_plain_format(
            head in "[a-z0-9_]{0,6}", sep in "[/.~\\\\]", tail in "[a-z0-9_./]{0,6}",
        ) {
            let value = format!("{head}{sep}{tail}");
            prop_assert!(!is_plain_format(&value));
            prop_assert!(!is_reader_format(&value));
            prop_assert!(!is_writer_format(&value));
        }

        #[test]
        fn a_plain_name_with_extensions_is_accepted_unless_it_is_pdf(
            name in "[a-z][a-z0-9_]{0,8}", ext in "[a-z][a-z0-9_]{0,8}", sign in "[+-]",
        ) {
            prop_assume!(name != "pdf");
            prop_assert!(is_plain_format(&name));
            let with_extension = format!("{name}{sign}{ext}");
            prop_assert!(is_plain_format(&with_extension));
        }

        #[test]
        fn a_lua_suffixed_writer_is_refused_in_every_spelling(
            prefix in "[a-z0-9_]{0,6}",
            ext in "([+-][a-z][a-z0-9_]{0,6}){0,2}",
            flag in prop::sample::select(vec!["-t", "-w", "-D", "-st", "--to", "--write", "--print-default-template"]),
            glued in any::<bool>(),
        ) {
            let value = format!("{prefix}lua{ext}");
            prop_assert!(!is_writer_format(&value));
            let cmd = if flag.starts_with("--") && glued {
                format!("pandoc {flag}={value} README.md")
            } else if !flag.starts_with("--") && glued {
                format!("pandoc {flag}{value} README.md")
            } else {
                format!("pandoc {flag} {value} README.md")
            };
            prop_assert!(!crate::is_safe_command(&cmd), "{}", cmd);
        }

        #[test]
        fn pdf_is_refused_with_any_extensions(ext in "([+-][a-z][a-z0-9_]{0,6}){0,3}") {
            let value = format!("pdf{ext}");
            prop_assert!(!is_plain_format(&value));
            prop_assert!(!is_writer_format(&value));
        }

        #[test]
        fn a_writer_flag_with_a_path_value_is_denied_in_every_spelling(
            stem in "[a-z]{1,6}", flag in prop::sample::select(vec!["-t", "-w", "-f", "-r", "-D", "--to", "--write", "--from", "--read", "--print-default-template"]),
            glued in any::<bool>(),
        ) {
            let value = format!("./{stem}.lua");
            let cmd = if flag.starts_with("--") {
                format!("pandoc {flag}={value} README.md")
            } else if glued {
                format!("pandoc {flag}{value} README.md")
            } else {
                format!("pandoc {flag} {value} README.md")
            };
            prop_assert!(!crate::is_safe_command(&cmd), "{}", cmd);
        }
    }
}
