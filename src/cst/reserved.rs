pub(super) const BLANK_OPENERS: [&str; 2] = ["{", "[["];
pub(super) const KEYWORD_OPENERS: [&str; 6] = ["if", "for", "while", "until", "case", "function"];

/// Whether `input`, at a command position, begins with a word the shell reserves for a compound
/// command, so that only a compound parser may consume it.
///
/// Bash recognises `{`, `[[`, `if`, `for`, `while`, `until`, `case` and `function` as reserved
/// words when they stand as a whole token where a command starts. There is no reading in which one
/// of them is a command name: `{ ls` with no closing brace is a syntax error, not a call to a
/// program named `{`. The parser used to offer every such word to `simple_cmd` after the compound
/// parser failed, and that second reading is what made unclosed nesting exponential. Each
/// `{` level was parsed once as a brace group that failed at the far end of the input and once
/// more as a simple command named `{`, which re-entered the next level twice again. Twenty
/// unclosed braces in front of a ~1.4 KB tail ran each parse into its work budget, and a
/// classification that re-parsed the backtick body several times took 4s.
///
/// The fallback never changed a verdict. A command named `{`, `if` or `[[` resolves to nothing and
/// is refused, so committing turns "parsed, refused" into "did not parse, refused".
///
/// `{` and `[[` need a following blank, the same test their parsers apply, so `{a,b}` and
/// `[[x` stay ordinary words. A keyword needs to end at a shell metacharacter, which is where a
/// token ends: `if=1 cmd` and `for_each` are an assignment and a command name, not keywords.
pub(super) fn opens_compound(input: &str) -> bool {
    for opener in BLANK_OPENERS {
        if let Some(rest) = input.strip_prefix(opener) {
            return rest.starts_with([' ', '\t', '\n']);
        }
    }
    KEYWORD_OPENERS.iter().any(|kw| {
        input.strip_prefix(kw).is_some_and(|rest| {
            rest.is_empty() || rest.starts_with([' ', '\t', '\n', ';', '&', '|', '(', ')', '<', '>'])
        })
    })
}

#[cfg(test)]
mod tests {
    use super::opens_compound;

    #[test]
    fn a_reserved_word_standing_alone_opens_a_compound() {
        for input in [
            "{ ls; }", "{\nls\n}", "[[ -f x ]]", "if true; then ls; fi", "for x in a; do ls; done",
            "while true; do ls; done", "until true; do ls; done", "case x in a) ls;; esac",
            "function f { ls; }", "if", "if;", "for(", "case\tx",
        ] {
            assert!(opens_compound(input), "{input:?} should commit to a compound");
        }
    }

    #[test]
    fn a_word_that_only_starts_like_one_does_not() {
        for input in [
            "{a,b}", "{}", "{", "[[x", "[", "if=1 ls", "for_each", "iffy", "cases", "functions",
            "whiled", "ls {", "echo if", "", "done",
        ] {
            assert!(!opens_compound(input), "{input:?} is not a reserved word");
        }
    }
}
