//! What a word can turn into at run time, read from its parts.

use super::items::substitution_shape;
use super::{BLANK, GLOB, Lead, OPAQUE, Parts, SPLIT, Shape, VANISH};
use crate::cst::eval::eval_part;
use crate::cst::{Script, Word, WordPart};
use crate::pathctx::item_shape::{self, EMPTY, ItemShape, LEAD, UNKNOWN};
use crate::pathctx::{Binding, Facts};

/// Variables the session sets to an absolute path or a login name, neither of which begins with
/// `-` or holds a blank. Trusted as the environment itself is (`$PATH` decides what `ls` runs); a
/// rebinding inside the command is a binding, not this.
pub(super) const TRUSTED_VARS: &[&str] = &["HOME", "LOGNAME", "PWD", "TMPDIR", "USER"];

/// An unquoted expansion whose value holds one of these is globbed after it is substituted.
pub(super) const GLOB_CHARS: [char; 3] = ['*', '?', '['];

/// Marks the facts about how far a walk over a word has got.
#[derive(Debug, PartialEq, Eq)]
enum Progress {}

/// Nothing certainly non-empty has been read yet, so what comes next starts the word.
const AT_START: Facts<Progress> = Facts::bit(0);
/// An unknown part that may be empty came before anything known.
const AFTER_OPAQUE: Facts<Progress> = Facts::bit(1);
/// Every part so far may expand to nothing, so the whole word may vanish.
const VANISHABLE: Facts<Progress> = Facts::bit(2);
/// A variable with a certain value was substituted.
const SUBSTITUTED: Facts<Progress> = Facts::bit(3);
/// An unquoted substitution may split the concrete value.
const RESPLIT: Facts<Progress> = Facts::bit(4);
/// Something in the command may have set `IFS`, so an unquoted expansion splits at characters
/// nobody can name, and even a value known in full can come apart into a flag or another path.
const IFS_CHANGED: Facts<Progress> = Facts::bit(5);

/// What `word` can turn into at run time. `expanded` is how many tokens brace expansion made of
/// it; a fanned-out word holding anything unknown is treated as wholly unknown.
pub(crate) fn shape(word: &Word, expanded: usize) -> Shape {
    let ifs = !matches!(crate::pathctx::binding("IFS"), Binding::Unbound);
    let mut walk = Walk { state: AT_START | VANISHABLE | Facts::when(IFS_CHANGED, ifs), ..Walk::default() };
    walk.parts(&word.0, false);
    let mut shape = walk.shape;
    shape.facts.set(VANISH, walk.state.has(VANISHABLE) && shape.facts.has(OPAQUE));
    if walk.state.has(SUBSTITUTED) {
        let words = if walk.state.has(RESPLIT) { walk.text.split_whitespace().map(str::to_string).collect() } else { vec![walk.text] };
        shape.concrete = Some(words);
    }
    if expanded != 1 && (shape.facts.has(OPAQUE | GLOB) || shape.lead != Lead::No) {
        shape.lead = Lead::Any;
        shape.facts.set(SPLIT | VANISH, true);
    }
    shape
}

/// How an unknown part whose values have `items`'s shape behaves in a word, quoted or not.
fn in_word(items: ItemShape, quoted: bool) -> Facts<Parts> {
    let blank = items.has(item_shape::SPLIT);
    Facts::when(SPLIT, blank && !quoted) | Facts::when(VANISH, items.has(EMPTY) && !quoted) | Facts::when(BLANK, blank)
}

fn lead_of(items: ItemShape) -> Lead {
    if items.has(LEAD) { Lead::Any } else { Lead::No }
}

#[derive(Default)]
struct Walk {
    shape: Shape,
    text: String,
    state: Facts<Progress>,
}

impl Walk {
    fn parts(&mut self, parts: &[WordPart], quoted: bool) {
        for part in parts {
            match part {
                WordPart::Lit(s) => self.text_part(s, quoted),
                WordPart::Escape(c) => self.quoted_known(&c.to_string()),
                WordPart::SQuote(s) => self.quoted_known(s),
                WordPart::AnsiC(raw) => self.quoted_known(&crate::cst::ansi_c::decode(raw)),
                WordPart::DQuote(inner) => {
                    self.state.clear(VANISHABLE);
                    self.parts(&inner.0, true);
                }
                WordPart::CmdSub(script) => self.substitution(Some(script), quoted, part),
                WordPart::Backtick(raw) => self.substitution(crate::cst::parse(raw).as_ref(), quoted, part),
                WordPart::Arith(body) if nonnegative_constant(body) => self.known(&spelled(part)),
                WordPart::Arith(_) => self.opaque(Lead::Number, Facts::when(SPLIT, self.resplits(quoted)), &spelled(part)),
                WordPart::ProcSub(_) => self.known(&spelled(part)),
            }
        }
    }

    fn substitution(&mut self, script: Option<&Script>, quoted: bool, part: &WordPart) {
        let items = if self.resplits(quoted) { UNKNOWN } else { script.map_or(UNKNOWN, substitution_shape) };
        self.opaque(lead_of(items), in_word(items, quoted), &spelled(part));
    }

    fn quoted_known(&mut self, s: &str) {
        self.state.clear(VANISHABLE);
        self.known(s);
    }

    fn known(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        if self.state.has(AT_START) && self.state.has(AFTER_OPAQUE) && s.starts_with('-') {
            self.lead(Lead::Any);
        }
        self.text.push_str(s);
        self.state.clear(AT_START | VANISHABLE);
    }

    /// An unknown part. `facts` says whether it may split ([`SPLIT`]), expand to nothing
    /// ([`VANISH`]) or hold a blank ([`BLANK`]).
    fn opaque(&mut self, lead: Lead, facts: Facts<Parts>, spelled: &str) {
        self.shape.facts.set(OPAQUE, true);
        self.shape.facts.set(SPLIT, facts.has(SPLIT));
        self.shape.facts.set(BLANK, facts.has(BLANK));
        if self.state.has(AT_START) {
            self.lead(lead);
            self.state.set(AFTER_OPAQUE, true);
        }
        if !facts.has(VANISH) {
            self.state.clear(VANISHABLE);
        }
        self.text.push_str(spelled);
    }

    fn resplits(&self, quoted: bool) -> bool {
        !quoted && self.state.has(IFS_CHANGED)
    }

    fn lead(&mut self, lead: Lead) {
        self.shape.lead = self.shape.lead.max(lead);
    }

    fn text_part(&mut self, s: &str, quoted: bool) {
        let mut i = 0;
        while let Some(c) = s[i..].chars().next() {
            if c == '$'
                && let Some(used) = self.dollar(&s[i + 1..], quoted)
            {
                i += 1 + used;
                continue;
            }
            if !quoted {
                self.glob(c, &s[i + c.len_utf8()..]);
            }
            self.known(&s[i..i + c.len_utf8()]);
            i += c.len_utf8();
        }
    }

    /// A glob that can match a name beginning with `-`, or a brace sequence that can count
    /// through negative numbers. Either may also become several words.
    fn glob(&mut self, c: char, rest: &str) {
        let pattern = matches!(c, '*' | '?') || (c == '[' && rest.contains(']'));
        self.shape.facts.set(GLOB, pattern);
        let sequence = c == '{' && rest.find('}').is_some_and(|close| rest[..close].contains(".."));
        if self.state.has(AT_START) && (pattern || sequence) {
            self.lead(if pattern { Lead::Any } else { Lead::Number });
            self.shape.facts.set(SPLIT, true);
            self.state.set(AFTER_OPAQUE, true);
        }
    }

    /// The expansion after a `$`, or `None` when the `$` is a literal one. Returns the bytes used.
    fn dollar(&mut self, rest: &str, quoted: bool) -> Option<usize> {
        let first = rest.chars().next()?;
        let many = |all: bool| Facts::when(SPLIT | VANISH, all) | BLANK;
        match first {
            '{' => {
                let Some(close) = rest.find('}') else {
                    self.opaque(Lead::Any, many(true), &format!("${rest}"));
                    return Some(rest.len());
                };
                let inner = &rest[1..close];
                let spelled = format!("${}", &rest[..=close]);
                if is_name(inner) {
                    self.var(inner, quoted, &spelled);
                } else if inner.strip_prefix('#').is_some_and(is_name) {
                    self.known(&spelled);
                } else {
                    self.opaque(Lead::Any, many(!quoted || inner.contains('@')), &spelled);
                }
                Some(close + 1)
            }
            '0'..='9' => {
                self.var(&rest[..1], quoted, &format!("${first}"));
                Some(1)
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let len = rest.bytes().take_while(|b| b.is_ascii_alphanumeric() || *b == b'_').count();
                self.var(&rest[..len], quoted, &format!("${}", &rest[..len]));
                Some(len)
            }
            '@' | '*' => {
                self.opaque(Lead::Any, many(!quoted || first == '@'), &format!("${first}"));
                Some(1)
            }
            '#' | '?' | '$' | '-' => {
                self.known(&format!("${first}"));
                Some(1)
            }
            '!' => {
                self.opaque(Lead::Number, Facts::when(VANISH, !quoted), "$!");
                Some(1)
            }
            _ => None,
        }
    }

    fn var(&mut self, name: &str, quoted: bool, spelled: &str) {
        if self.resplits(quoted) {
            self.opaque(Lead::Any, in_word(UNKNOWN, quoted), spelled);
            return;
        }
        match crate::pathctx::binding(name) {
            Binding::Loop => {
                let items = crate::pathctx::loop_shape(name);
                self.opaque(lead_of(items), in_word(items, quoted), spelled);
            }
            Binding::Value(value)
                if !value.contains('$') && !value.contains("__SAFE_CHAINS_") && (quoted || !value.starts_with(GLOB_CHARS)) =>
            {
                self.state.set(SUBSTITUTED, true);
                self.state.set(RESPLIT, !quoted && (value.is_empty() || value.contains(char::is_whitespace)));
                self.known(&value);
            }
            Binding::Unbound if TRUSTED_VARS.contains(&name) => self.known(spelled),
            _ => self.opaque(Lead::Any, in_word(UNKNOWN, quoted), spelled),
        }
    }
}

fn spelled(part: &WordPart) -> String {
    let mut out = String::new();
    eval_part(part, &mut out);
    out
}

fn is_name(s: &str) -> bool {
    let mut bytes = s.bytes();
    match bytes.next() {
        Some(b) if b.is_ascii_digit() => s.bytes().all(|b| b.is_ascii_digit()),
        Some(b) if b.is_ascii_alphabetic() || b == b'_' => bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_'),
        _ => false,
    }
}

/// Arithmetic that can only be a non-negative number: digits, blanks and `+`, short enough that
/// no sum overflows into the negatives. Anything else (a variable, `-`, `*`, a shift) may be
/// negative, which reads as a flag.
fn nonnegative_constant(body: &Word) -> bool {
    let text = body.eval();
    text.len() <= 18
        && body.0.iter().all(|p| matches!(p, WordPart::Lit(_)))
        && text.chars().all(|c| c.is_ascii_digit() || c == '+' || c.is_ascii_whitespace())
}
