use super::{Word, WordPart};

pub fn eval_word(word: &Word) -> String {
    let mut out = String::new();
    for part in &word.0 {
        eval_part(part, &mut out);
    }
    out
}

/// Past this many words from one expansion we stop enumerating and fail CLOSED — a hostile
/// `{a,b}{a,b}…` would otherwise blow up 2^n. The word becomes an unpinnable sentinel (→ machine
/// → deny) rather than a DoS.
const BRACE_EXPANSION_CAP: usize = 256;
const UNPINNABLE: &str = "__SAFE_CHAINS_CMDSUB__";

/// Expand a word to the literal words bash would produce, chiefly via UNQUOTED brace expansion
/// (`{/etc/shadow,x}` → two words). Quoted / escaped / substituted parts are fixed (bash does not
/// brace-expand inside quotes), so only `Lit` parts expand. The classifier checks EVERY produced
/// word, so a hidden alternative can't smuggle a system read/write past the `/`-prefix path gate.
pub fn expand_word(word: &Word) -> Vec<String> {
    let mut acc = vec![String::new()];
    for part in &word.0 {
        let alts: Vec<String> = match part {
            WordPart::Lit(s) => brace_expand(s),
            other => {
                let mut t = String::new();
                eval_part(other, &mut t);
                vec![t]
            }
        };
        if alts.len() > BRACE_EXPANSION_CAP || acc.len().saturating_mul(alts.len()) > BRACE_EXPANSION_CAP {
            return vec![UNPINNABLE.to_string()];
        }
        let mut next = Vec::with_capacity(acc.len() * alts.len());
        for a in &acc {
            for b in &alts {
                next.push(format!("{a}{b}"));
            }
        }
        acc = next;
    }
    acc
}

/// Brace-expand a literal's comma groups: `a{1,2}b` → [`a1b`, `a2b`]; nested groups recurse. A
/// `{...}` with no top-level comma is literal (bash: `{x}` stays `{x}`); sequence forms (`{1..5}`)
/// are left literal (they can't name a hot path). Fails closed to an unpinnable sentinel if the
/// word carries a pathological number of brace groups.
fn brace_expand(s: &str) -> Vec<String> {
    if s.matches('{').count() > 8 {
        return vec![UNPINNABLE.to_string()];
    }
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '{' {
            let mut depth = 1;
            let mut j = i + 1;
            let mut comma = false;
            while j < chars.len() {
                match chars[j] {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    ',' if depth == 1 => comma = true,
                    _ => {}
                }
                j += 1;
            }
            // A closed group with a top-level comma expands; otherwise skip this `{` (it is
            // literal, e.g. `{x}` or an unclosed brace) and look for a later group.
            if j < chars.len() && depth == 0 && comma {
                let prefix: String = chars[..i].iter().collect();
                let content: String = chars[i + 1..j].iter().collect();
                let suffix: String = chars[j + 1..].iter().collect();
                let mut out = Vec::new();
                for alt in split_top_commas(&content) {
                    for alt_x in brace_expand(&alt) {
                        for suf_x in brace_expand(&suffix) {
                            out.push(format!("{prefix}{alt_x}{suf_x}"));
                        }
                    }
                }
                return out;
            }
        }
        i += 1;
    }
    vec![s.to_string()]
}

/// Split on commas at brace-nesting depth 0, so `a,{b,c},d` → [`a`, `{b,c}`, `d`].
fn split_top_commas(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '{' => {
                depth += 1;
                cur.push(c);
            }
            '}' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => parts.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    parts.push(cur);
    parts
}

fn eval_part(part: &WordPart, out: &mut String) {
    match part {
        WordPart::Lit(s) => out.push_str(s),
        WordPart::Escape(c) => out.push(*c),
        WordPart::SQuote(s) => out.push_str(s),
        WordPart::DQuote(inner) => {
            for p in &inner.0 {
                eval_part(p, out);
            }
        }
        // Command substitution / backtick: the output becomes the operand VALUE — an
        // unknowable path, so the classifier must worst-case it (see locus::is_unpinnable).
        WordPart::CmdSub(_) | WordPart::Backtick(_) => out.push_str("__SAFE_CHAINS_CMDSUB__"),
        // Process substitution: the operand is a /dev/fd pipe; its safety is the INNER command
        // (checked separately by word_sub_verdict), not an unknowable path — so a distinct
        // placeholder that classifies as an ordinary (worktree) operand.
        WordPart::ProcSub(_) => out.push_str("__SAFE_CHAINS_PROCSUB__"),
        WordPart::Arith(_) => out.push_str("__SAFE_CHAINS_ARITH__"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal() {
        let w = Word(vec![WordPart::Lit("hello".into())]);
        assert_eq!(w.eval(), "hello");
    }

    #[test]
    fn single_quoted() {
        let w = Word(vec![WordPart::SQuote("hello world".into())]);
        assert_eq!(w.eval(), "hello world");
    }

    #[test]
    fn double_quoted() {
        let w = Word(vec![WordPart::DQuote(Word(vec![
            WordPart::Lit("hello ".into()),
            WordPart::Lit("world".into()),
        ]))]);
        assert_eq!(w.eval(), "hello world");
    }

    #[test]
    fn mixed_parts() {
        let w = Word(vec![
            WordPart::Lit("foo".into()),
            WordPart::DQuote(Word(vec![WordPart::Lit("bar".into())])),
            WordPart::SQuote("baz".into()),
        ]);
        assert_eq!(w.eval(), "foobarbaz");
    }

    #[test]
    fn cmd_sub_placeholder() {
        let w = Word(vec![WordPart::CmdSub(super::super::Script(vec![]))]);
        assert_eq!(w.eval(), "__SAFE_CHAINS_CMDSUB__");
    }

    #[test]
    fn dquote_with_escape() {
        let w = Word(vec![WordPart::DQuote(Word(vec![
            WordPart::Lit("hello".into()),
            WordPart::Escape('"'),
            WordPart::Lit("world".into()),
        ]))]);
        assert_eq!(w.eval(), "hello\"world");
    }
}
