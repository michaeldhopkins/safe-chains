//! `pandoc` converts documents to stdout, with three spellings that run code or a program:
//! `pandoc lua` is a standalone Lua interpreter, `pandoc server` listens on a port, and the format
//! flags (`-f`/`-r` reader, `-t`/`-w` writer, `-D` template) load a Lua script when the value is a
//! path. `-t pdf` runs TeX. The first word and the format values are checked here; every other flag
//! is checked against the lists in `commands/tools/pandoc.toml`.
use crate::parse::Token;
use crate::policy::FlagSet;
use crate::registry;
use crate::verdict::Verdict;

const FORMAT_LONG: &[&str] = &["--from", "--read", "--to", "--write", "--print-default-template"];
const FORMAT_SHORT: &[u8] = b"frtwD";

pub fn check_pandoc(tokens: &[Token]) -> Verdict {
    if matches!(tokens.get(1).map(Token::as_str), Some("lua" | "server")) {
        return Verdict::Denied;
    }
    if !format_values_are_plain(tokens) {
        return Verdict::Denied;
    }
    registry::try_fallback_grammar("pandoc", tokens).unwrap_or(Verdict::Denied)
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
            let takes_value = valued.contains_flag(&name) || FORMAT_LONG.contains(&name.as_str());
            if inline.is_none() && takes_value {
                let value = tokens.get(i).map(Token::as_str);
                i += 1;
                if FORMAT_LONG.contains(&name.as_str()) && !value.is_some_and(is_plain_format) {
                    return false;
                }
            } else if let Some(value) = inline
                && FORMAT_LONG.contains(&name.as_str())
                && !is_plain_format(value)
            {
                return false;
            }
            continue;
        }
        if !t.starts_with('-') || t == "-" {
            continue;
        }
        let bytes = t.as_bytes();
        for (j, &b) in bytes.iter().enumerate().skip(1) {
            if standalone.contains_short(b) {
                continue;
            }
            let is_format = FORMAT_SHORT.contains(&b);
            if is_format || valued.contains_short(b) {
                let value = if j + 1 < bytes.len() {
                    t.get(j + 1..)
                } else {
                    i += 1;
                    tokens.get(i - 1).map(Token::as_str)
                };
                if is_format && !value.is_some_and(is_plain_format) {
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
    use proptest::prelude::*;

    use super::is_plain_format;

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
    }

    denied! {
        lua_interpreter: "pandoc lua x.lua",
        server: "pandoc server",
        lua_writer: "pandoc -t ./w.lua README.md",
        lua_reader_long: "pandoc --from=./r.lua README.md",
        lua_template: "pandoc -D d.lua",
        pdf_writer: "pandoc -t pdf README.md",
        missing_value: "pandoc README.md -t",
        equals_form_short: "pandoc -t=html README.md",
        trailing_separator: "pandoc -t html+ README.md",
    }

    #[test]
    fn a_plain_format_is_read_only() {
        assert_eq!(crate::command_verdict("pandoc -t html README.md"), Verdict::Allowed(SafetyLevel::SafeRead));
    }

    proptest! {
        #[test]
        fn a_value_with_a_path_character_is_never_a_plain_format(
            head in "[a-z0-9_]{0,6}", sep in "[/.~\\\\]", tail in "[a-z0-9_./]{0,6}",
        ) {
            let value = format!("{head}{sep}{tail}");
            prop_assert!(!is_plain_format(&value));
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
        fn pdf_is_refused_with_any_extensions(ext in "([+-][a-z][a-z0-9_]{0,6}){0,3}") {
            let value = format!("pdf{ext}");
            prop_assert!(!is_plain_format(&value));
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
