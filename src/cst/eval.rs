use super::{Word, WordPart};

pub fn eval_word(word: &Word) -> String {
    let mut out = String::new();
    for part in &word.0 {
        eval_part(part, &mut out);
    }
    out
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
        WordPart::CmdSub(_) => out.push_str("__SAFE_CHAINS_SUB__"),
        WordPart::Backtick(_) => out.push_str("__SAFE_CHAINS_SUB__"),
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
        assert_eq!(w.eval(), "__SAFE_CHAINS_SUB__");
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
